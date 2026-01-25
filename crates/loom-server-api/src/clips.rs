// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! API types for the clips (code snippets) system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Visibility level for a clip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "lowercase")]
pub enum ClipVisibilityApi {
	/// Only the owner can view.
	#[default]
	Private,
	/// Anyone in the org can view.
	Internal,
	/// Anyone with the link can view.
	Public,
}

/// Request to create a new clip.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateClipRequest {
	/// Clip name (unique per owner).
	pub name: String,
	/// Optional description.
	pub description: Option<String>,
	/// Visibility level.
	#[serde(default)]
	pub visibility: ClipVisibilityApi,
	/// Optional organization ID (if creating for an org).
	pub org_id: Option<Uuid>,
}

/// Request to update a clip.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateClipRequest {
	/// New name (optional).
	pub name: Option<String>,
	/// New description (optional).
	pub description: Option<String>,
	/// New visibility (optional).
	pub visibility: Option<ClipVisibilityApi>,
}

/// Request to fork a clip.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ForkClipRequest {
	/// New name for the forked clip (optional, defaults to original name).
	pub name: Option<String>,
	/// Optional organization ID (if forking to an org).
	pub org_id: Option<Uuid>,
}

/// Clip response with metadata.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ClipResponse {
	/// Unique identifier.
	pub id: Uuid,
	/// Owner username or organization name.
	pub owner: String,
	/// Clip name.
	pub name: String,
	/// Optional description.
	pub description: Option<String>,
	/// Visibility level.
	pub visibility: ClipVisibilityApi,
	/// User ID who created the clip.
	pub created_by: Uuid,
	/// Organization ID if org-owned.
	pub org_id: Option<Uuid>,
	/// Whether this is a fork.
	pub is_fork: bool,
	/// Original clip ID if forked.
	pub forked_from: Option<Uuid>,
	/// Number of files.
	pub file_count: u32,
	/// Total size in bytes.
	pub size_bytes: u64,
	/// Primary language.
	pub language: Option<String>,
	/// Clone URL for git operations.
	pub clone_url: String,
	/// Creation timestamp.
	pub created_at: DateTime<Utc>,
	/// Last update timestamp.
	pub updated_at: DateTime<Utc>,
}

/// List clips response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListClipsResponse {
	pub clips: Vec<ClipResponse>,
}

/// File within a clip.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ClipFileResponse {
	/// File path within the clip.
	pub path: String,
	/// File content (may be redacted).
	pub content: String,
	/// Size in bytes.
	pub size_bytes: u64,
	/// Whether the content has been redacted.
	pub is_redacted: bool,
	/// Detected language/syntax.
	pub language: Option<String>,
}

/// List files in a clip response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListClipFilesResponse {
	pub files: Vec<String>,
}

/// Clips success response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ClipsSuccessResponse {
	pub message: String,
}

/// Clips error response.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ClipsErrorResponse {
	pub error: String,
	pub message: String,
}

/// Query parameters for listing clips.
#[derive(Debug, Deserialize, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListClipsQuery {
	/// Maximum number of clips to return.
	#[serde(default = "default_limit")]
	pub limit: u32,
	/// Offset for pagination.
	#[serde(default)]
	pub offset: u32,
	/// Filter by language.
	pub language: Option<String>,
	/// Filter by visibility.
	pub visibility: Option<ClipVisibilityApi>,
}

fn default_limit() -> u32 {
	50
}
