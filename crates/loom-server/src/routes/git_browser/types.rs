// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Types for Git browser API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct BrowserRepoResponse {
	pub id: Uuid,
	pub owner_type: String,
	pub owner_id: Uuid,
	pub name: String,
	pub visibility: String,
	pub default_branch: String,
	pub clone_url: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TreeEntry {
	pub name: String,
	pub path: String,
	pub kind: String,
	pub sha: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub size: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Branch {
	pub name: String,
	pub sha: String,
	pub is_default: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommitInfo {
	pub sha: String,
	pub message: String,
	pub author_name: String,
	pub author_email: String,
	pub author_date: String,
	pub parent_shas: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommitWithDiff {
	#[serde(flatten)]
	pub commit: CommitInfo,
	pub diff: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListCommitsResponse {
	pub commits: Vec<CommitInfo>,
	pub total: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BlameLine {
	pub line_number: usize,
	pub commit_sha: String,
	pub author_name: String,
	pub author_email: String,
	pub author_date: String,
	pub content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompareResult {
	pub base_ref: String,
	pub head_ref: String,
	pub commits: Vec<CommitInfo>,
	pub diff: String,
	pub ahead_by: usize,
	pub behind_by: usize,
}

#[derive(Debug, Deserialize)]
pub struct ListCommitsParams {
	pub limit: Option<usize>,
	pub offset: Option<usize>,
	pub path: Option<String>,
}
