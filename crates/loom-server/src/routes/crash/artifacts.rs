// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash artifact HTTP handlers.
//!
//! Implements endpoints for symbol/source map artifact management.

use axum::{
	extract::{Multipart, Path, Query, State},
	http::StatusCode,
	Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use loom_crash_core::{ArtifactType, ProjectId, SymbolArtifact, SymbolArtifactId, UserId};
use loom_server_audit::{AuditEventType, AuditLogBuilder, UserId as AuditUserId};
use loom_server_crash::CrashRepository;

use crate::api::AppState;
use crate::auth_middleware::RequireAuth;
use crate::i18n::resolve_user_locale;

use super::common::{
	internal_error, not_found, parse_project_id, verify_org_membership, CrashErrorResponse,
};

// ============================================================================
// Request/Response Types
// ============================================================================

/// Response for artifact operations.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ArtifactResponse {
	pub id: String,
	pub project_id: String,
	pub release: String,
	pub dist: Option<String>,
	pub artifact_type: String,
	pub name: String,
	pub size_bytes: u64,
	pub sha256: String,
	pub source_map_url: Option<String>,
	pub sources_content: bool,
	pub uploaded_at: String,
	pub uploaded_by: String,
	pub last_accessed_at: Option<String>,
}

impl From<SymbolArtifact> for ArtifactResponse {
	fn from(a: SymbolArtifact) -> Self {
		Self {
			id: a.id.to_string(),
			project_id: a.project_id.to_string(),
			release: a.release,
			dist: a.dist,
			artifact_type: a.artifact_type.to_string(),
			name: a.name,
			size_bytes: a.size_bytes,
			sha256: a.sha256,
			source_map_url: a.source_map_url,
			sources_content: a.sources_content,
			uploaded_at: a.uploaded_at.to_rfc3339(),
			uploaded_by: a.uploaded_by.to_string(),
			last_accessed_at: a.last_accessed_at.map(|dt| dt.to_rfc3339()),
		}
	}
}

/// Response for artifact upload operations.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct UploadArtifactResponse {
	/// Total number of files in the request
	pub total: usize,
	/// Number of newly uploaded files
	pub uploaded_count: usize,
	/// Number of files that already existed (deduplicated)
	pub existing_count: usize,
	/// Number of files that failed to upload
	pub error_count: usize,
	/// List of artifacts (both new and existing)
	pub artifacts: Vec<ArtifactResponse>,
	/// Errors for files that failed
	pub errors: Vec<ArtifactUploadError>,
}

/// Error for a single artifact upload.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ArtifactUploadError {
	pub filename: String,
	pub error: String,
}

/// Query parameters for listing artifacts.
#[derive(Debug, Deserialize)]
pub struct ListArtifactsParams {
	pub release: Option<String>,
	#[serde(default = "default_artifact_limit")]
	pub limit: u32,
}

fn default_artifact_limit() -> u32 {
	100
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse an artifact ID from a string.
fn parse_artifact_id(
	artifact_id_str: &str,
) -> Result<SymbolArtifactId, (StatusCode, Json<CrashErrorResponse>)> {
	artifact_id_str.parse().map_err(|_| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_artifact_id".to_string(),
				message: "Invalid artifact ID".to_string(),
			}),
		)
	})
}

/// Get a project and verify org membership.
async fn get_project_with_auth(
	state: &AppState,
	project_id: ProjectId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<loom_crash_core::CrashProject, (StatusCode, Json<CrashErrorResponse>)> {
	let project = state
		.crash_repo
		.get_project_by_id(project_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get project");
			internal_error(locale)
		})?
		.ok_or_else(|| not_found("project"))?;

	verify_org_membership(state, &project.org_id, user_id, locale).await?;

	Ok(project)
}

// ============================================================================
// Artifact Endpoints
// ============================================================================

/// POST /api/crash/projects/{project_id}/artifacts - Upload artifacts (multipart)
///
/// Upload source maps and minified source files for a release. Files are deduplicated
/// by SHA256 hash. The multipart form should include:
/// - `release` (text field): The release version these artifacts belong to
/// - `dist` (optional text field): Distribution variant
/// - Files: One or more source map (.map) or JavaScript (.js) files
#[utoipa::path(
	post,
	path = "/api/crash/projects/{project_id}/artifacts",
	params(
		("project_id" = String, Path, description = "Project ID"),
	),
	responses(
		(status = 200, description = "Upload results", body = UploadArtifactResponse),
		(status = 400, description = "Invalid request", body = CrashErrorResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user, multipart))]
pub async fn upload_artifacts(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
	mut multipart: Multipart,
) -> Result<Json<UploadArtifactResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Parse multipart form
	let mut release: Option<String> = None;
	let mut dist: Option<String> = None;
	let mut files: Vec<(String, Vec<u8>)> = Vec::new();

	while let Some(field) = multipart.next_field().await.map_err(|e| {
		tracing::error!(error = %e, "Failed to read multipart field");
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_multipart".to_string(),
				message: format!("Failed to read multipart data: {}", e),
			}),
		)
	})? {
		let name = field.name().map(|s| s.to_string());
		let file_name = field.file_name().map(|s| s.to_string());

		match name.as_deref() {
			Some("release") => {
				let bytes = field.bytes().await.map_err(|e| {
					(
						StatusCode::BAD_REQUEST,
						Json(CrashErrorResponse {
							error: "invalid_multipart".to_string(),
							message: format!("Failed to read release field: {}", e),
						}),
					)
				})?;
				release = Some(String::from_utf8_lossy(&bytes).to_string());
			}
			Some("dist") => {
				let bytes = field.bytes().await.map_err(|e| {
					(
						StatusCode::BAD_REQUEST,
						Json(CrashErrorResponse {
							error: "invalid_multipart".to_string(),
							message: format!("Failed to read dist field: {}", e),
						}),
					)
				})?;
				let dist_str = String::from_utf8_lossy(&bytes).to_string();
				if !dist_str.is_empty() {
					dist = Some(dist_str);
				}
			}
			_ => {
				// Assume it's a file
				if let Some(filename) = file_name {
					let data = field.bytes().await.map_err(|e| {
						(
							StatusCode::BAD_REQUEST,
							Json(CrashErrorResponse {
								error: "invalid_multipart".to_string(),
								message: format!("Failed to read file {}: {}", filename, e),
							}),
						)
					})?;
					files.push((filename, data.to_vec()));
				}
			}
		}
	}

	// Validate release is provided
	let release = release.ok_or_else(|| {
		(
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "missing_release".to_string(),
				message: "The 'release' field is required".to_string(),
			}),
		)
	})?;

	if release.is_empty() || release.len() > 200 {
		return Err((
			StatusCode::BAD_REQUEST,
			Json(CrashErrorResponse {
				error: "invalid_release".to_string(),
				message: "Release must be 1-200 characters".to_string(),
			}),
		));
	}

	// Process files
	let mut artifacts = Vec::new();
	let mut errors = Vec::new();
	let mut uploaded_count = 0;
	let mut existing_count = 0;
	let total = files.len();

	for (filename, data) in files {
		// Compute SHA256
		use sha2::{Digest, Sha256};
		let mut hasher = Sha256::new();
		hasher.update(&data);
		let sha256 = format!("{:x}", hasher.finalize());

		// Check for existing artifact by hash
		match state
			.crash_repo
			.get_artifact_by_sha256(project_id, &sha256)
			.await
		{
			Ok(Some(existing)) => {
				info!(
					artifact_id = %existing.id,
					filename = %filename,
					sha256 = %sha256,
					"Artifact already exists (deduplicated)"
				);
				artifacts.push(ArtifactResponse::from(existing));
				existing_count += 1;
				continue;
			}
			Ok(None) => {}
			Err(e) => {
				tracing::warn!(error = %e, filename = %filename, "Failed to check for existing artifact");
				errors.push(ArtifactUploadError {
					filename,
					error: format!("Failed to check for duplicates: {}", e),
				});
				continue;
			}
		}

		// Determine artifact type
		let artifact_type = if filename.ends_with(".map") {
			ArtifactType::SourceMap
		} else {
			ArtifactType::MinifiedSource
		};

		// For source maps, check if sourcesContent is embedded
		let sources_content = if artifact_type == ArtifactType::SourceMap {
			// Simple check for sourcesContent in the JSON
			String::from_utf8_lossy(&data).contains("\"sourcesContent\"")
		} else {
			false
		};

		let artifact = SymbolArtifact {
			id: SymbolArtifactId::new(),
			org_id: project.org_id,
			project_id,
			release: release.clone(),
			dist: dist.clone(),
			artifact_type,
			name: filename.clone(),
			data: data.clone(),
			size_bytes: data.len() as u64,
			sha256,
			source_map_url: None,
			sources_content,
			uploaded_at: Utc::now(),
			uploaded_by: UserId(current_user.user.id.into_inner()),
			last_accessed_at: None,
		};

		match state.crash_repo.create_artifact(&artifact).await {
			Ok(()) => {
				info!(
					artifact_id = %artifact.id,
					filename = %filename,
					size_bytes = %artifact.size_bytes,
					"Artifact uploaded"
				);
				artifacts.push(ArtifactResponse::from(artifact));
				uploaded_count += 1;
			}
			Err(e) => {
				tracing::error!(error = %e, filename = %filename, "Failed to save artifact");
				errors.push(ArtifactUploadError {
					filename,
					error: format!("Failed to save: {}", e),
				});
			}
		}
	}

	info!(
		project_id = %project_id,
		release = %release,
		total,
		uploaded_count,
		existing_count,
		error_count = errors.len(),
		"Artifact upload completed"
	);

	if uploaded_count > 0 {
		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::CrashSymbolsUploaded)
				.actor(AuditUserId::new(current_user.user.id.into_inner()))
				.resource("crash_project", project_id.to_string())
				.details(serde_json::json!({
					"release": release,
					"total": total,
					"uploaded_count": uploaded_count,
					"existing_count": existing_count,
					"error_count": errors.len(),
				}))
				.build(),
		);
	}

	Ok(Json(UploadArtifactResponse {
		total,
		uploaded_count,
		existing_count,
		error_count: errors.len(),
		artifacts,
		errors,
	}))
}

/// GET /api/crash/projects/{project_id}/artifacts - List artifacts
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/artifacts",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("release" = Option<String>, Query, description = "Filter by release version"),
		("limit" = Option<u32>, Query, description = "Maximum number of artifacts to return (default 100)"),
	),
	responses(
		(status = 200, description = "List of artifacts", body = Vec<ArtifactResponse>),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Project not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn list_artifacts(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path(project_id_str): Path<String>,
	Query(params): Query<ListArtifactsParams>,
) -> Result<Json<Vec<ArtifactResponse>>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	let limit = params.limit.min(1000);
	let artifacts = state
		.crash_repo
		.list_artifacts(project_id, params.release.as_deref(), limit)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to list artifacts");
			internal_error(&locale)
		})?;

	info!(
		project_id = %project_id,
		artifact_count = %artifacts.len(),
		"Artifacts listed"
	);

	Ok(Json(
		artifacts.into_iter().map(ArtifactResponse::from).collect(),
	))
}

/// GET /api/crash/projects/{project_id}/artifacts/{artifact_id} - Get artifact metadata
#[utoipa::path(
	get,
	path = "/api/crash/projects/{project_id}/artifacts/{artifact_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("artifact_id" = String, Path, description = "Artifact ID"),
	),
	responses(
		(status = 200, description = "Artifact metadata", body = ArtifactResponse),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Artifact not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn get_artifact(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, artifact_id_str)): Path<(String, String)>,
) -> Result<Json<ArtifactResponse>, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let artifact_id = parse_artifact_id(&artifact_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	let artifact = state
		.crash_repo
		.get_artifact_by_id(artifact_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get artifact");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("artifact"))?;

	// Verify artifact belongs to project
	if artifact.project_id != project_id {
		return Err(not_found("artifact"));
	}

	// Update last_accessed_at
	let _ = state
		.crash_repo
		.update_artifact_last_accessed(artifact_id)
		.await;

	info!(
		artifact_id = %artifact.id,
		name = %artifact.name,
		"Artifact retrieved"
	);

	Ok(Json(ArtifactResponse::from(artifact)))
}

/// DELETE /api/crash/projects/{project_id}/artifacts/{artifact_id} - Delete artifact
#[utoipa::path(
	delete,
	path = "/api/crash/projects/{project_id}/artifacts/{artifact_id}",
	params(
		("project_id" = String, Path, description = "Project ID"),
		("artifact_id" = String, Path, description = "Artifact ID"),
	),
	responses(
		(status = 204, description = "Artifact deleted"),
		(status = 403, description = "Forbidden", body = CrashErrorResponse),
		(status = 404, description = "Artifact not found", body = CrashErrorResponse),
	),
	security(("bearer" = [])),
	tag = "crash"
)]
#[instrument(skip(state, current_user))]
pub async fn delete_artifact(
	State(state): State<AppState>,
	RequireAuth(current_user): RequireAuth,
	Path((project_id_str, artifact_id_str)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<CrashErrorResponse>)> {
	let locale = resolve_user_locale(&current_user, &state.default_locale);

	let project_id = parse_project_id(&project_id_str)?;
	let artifact_id = parse_artifact_id(&artifact_id_str)?;

	let _project = get_project_with_auth(&state, project_id, &current_user.user.id, &locale).await?;

	// Verify artifact exists and belongs to project
	let artifact = state
		.crash_repo
		.get_artifact_by_id(artifact_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to get artifact");
			internal_error(&locale)
		})?
		.ok_or_else(|| not_found("artifact"))?;

	if artifact.project_id != project_id {
		return Err(not_found("artifact"));
	}

	let deleted = state
		.crash_repo
		.delete_artifact(artifact_id)
		.await
		.map_err(|e| {
			tracing::error!(error = %e, "Failed to delete artifact");
			internal_error(&locale)
		})?;

	if deleted {
		info!(
			artifact_id = %artifact.id,
			name = %artifact.name,
			"Artifact deleted"
		);

		state.audit_service.log(
			AuditLogBuilder::new(AuditEventType::CrashSymbolsDeleted)
				.actor(AuditUserId::new(current_user.user.id.into_inner()))
				.resource("crash_artifact", artifact.id.to_string())
				.details(serde_json::json!({
					"project_id": project_id.to_string(),
					"name": artifact.name.clone(),
					"release": artifact.release.clone(),
				}))
				.build(),
		);

		Ok(StatusCode::NO_CONTENT)
	} else {
		Err(not_found("artifact"))
	}
}
