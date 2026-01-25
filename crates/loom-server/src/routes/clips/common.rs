// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for clips routes.

use loom_server_db::clips::{ClipRecord, ClipVisibility};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ============================================================================
// Request types
// ============================================================================

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateClipRequest {
	pub org_id: Option<Uuid>,
	pub name: String,
	pub description: Option<String>,
	pub visibility: Option<String>,
	#[serde(default)]
	pub files: Vec<CreateClipFile>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateClipFile {
	pub path: String,
	pub content: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateClipRequest {
	pub name: Option<String>,
	pub description: Option<String>,
	pub visibility: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ForkClipRequest {
	pub target_org_id: Uuid,
	pub name: Option<String>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListClipsQuery {
	pub page: Option<u32>,
	pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchClipsQuery {
	/// Search query
	pub q: String,
	pub page: Option<u32>,
	pub per_page: Option<u32>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFilesRequest {
	pub files: Vec<CreateClipFile>,
	pub message: Option<String>,
}

// ============================================================================
// Response types
// ============================================================================

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipResponse {
	pub id: Uuid,
	pub org_id: Option<Uuid>,
	pub owner: String,
	pub name: String,
	pub description: Option<String>,
	pub visibility: String,
	pub clone_url: String,
	pub file_count: u32,
	pub size_bytes: u64,
	pub language: Option<String>,
	pub star_count: u32,
	pub is_fork: bool,
	pub forked_from: Option<Uuid>,
	pub created_at: String,
	pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StarClipResponse {
	pub starred: bool,
	pub star_count: u32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipListResponse {
	pub clips: Vec<ClipResponse>,
	pub total: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipSearchHitResponse {
	pub clip: ClipResponse,
	pub score: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipSearchResponse {
	pub hits: Vec<ClipSearchHitResponse>,
	pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipFileResponse {
	pub path: String,
	pub content: String,
	pub size: u64,
	pub language: Option<String>,
	pub is_redacted: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipFilesResponse {
	pub files: Vec<ClipFileResponse>,
	pub revision: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipRevisionResponse {
	pub sha: String,
	pub author_name: String,
	pub author_email: String,
	pub timestamp: String,
	pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ClipRevisionsResponse {
	pub revisions: Vec<ClipRevisionResponse>,
}

// ============================================================================
// Helper functions
// ============================================================================

pub fn build_clip_url(base_url: &str, owner: &str, name: &str) -> String {
	format!("{}/git/clips/{}/{}.git", base_url.trim_end_matches('/'), owner, name)
}

pub fn clip_record_to_response(clip: ClipRecord, base_url: &str) -> ClipResponse {
	ClipResponse {
		id: clip.id,
		org_id: clip.org_id,
		owner: clip.owner.clone(),
		name: clip.name.clone(),
		description: clip.description,
		visibility: clip.visibility.to_string(),
		clone_url: build_clip_url(base_url, &clip.owner, &clip.name),
		file_count: clip.file_count,
		size_bytes: clip.size_bytes,
		language: clip.language,
		star_count: clip.star_count,
		is_fork: clip.is_fork,
		forked_from: clip.forked_from,
		created_at: clip.created_at.to_rfc3339(),
		updated_at: clip.updated_at.to_rfc3339(),
	}
}

pub fn parse_visibility(s: Option<&str>) -> ClipVisibility {
	match s {
		Some("public") => ClipVisibility::Public,
		Some("internal") => ClipVisibility::Internal,
		_ => ClipVisibility::Private,
	}
}

pub fn detect_language_from_path(path: &str) -> Option<String> {
	let ext = std::path::Path::new(path).extension()?.to_str()?;
	match ext.to_lowercase().as_str() {
		"rs" => Some("rust"),
		"js" | "jsx" | "mjs" => Some("javascript"),
		"ts" | "tsx" | "mts" => Some("typescript"),
		"py" => Some("python"),
		"go" => Some("go"),
		"java" => Some("java"),
		"c" | "h" => Some("c"),
		"cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
		"rb" => Some("ruby"),
		"php" => Some("php"),
		"swift" => Some("swift"),
		"kt" => Some("kotlin"),
		"sh" | "bash" => Some("shell"),
		"sql" => Some("sql"),
		"html" | "htm" => Some("html"),
		"css" => Some("css"),
		"json" => Some("json"),
		"yaml" | "yml" => Some("yaml"),
		"toml" => Some("toml"),
		"md" | "markdown" => Some("markdown"),
		"nix" => Some("nix"),
		"svelte" => Some("svelte"),
		_ => None,
	}
	.map(|s| s.to_string())
}

pub fn detect_content_type(path: &str) -> &'static str {
	let ext = std::path::Path::new(path)
		.extension()
		.and_then(|e| e.to_str())
		.unwrap_or("");
	match ext.to_lowercase().as_str() {
		"html" | "htm" => "text/html; charset=utf-8",
		"css" => "text/css; charset=utf-8",
		"js" | "mjs" => "application/javascript; charset=utf-8",
		"json" => "application/json; charset=utf-8",
		"xml" => "application/xml; charset=utf-8",
		"png" => "image/png",
		"jpg" | "jpeg" => "image/jpeg",
		"gif" => "image/gif",
		"svg" => "image/svg+xml",
		"pdf" => "application/pdf",
		"zip" => "application/zip",
		"tar" => "application/x-tar",
		"gz" => "application/gzip",
		_ => "text/plain; charset=utf-8",
	}
}
