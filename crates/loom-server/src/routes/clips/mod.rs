// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Clips (code snippets) HTTP handlers.
//!
//! This module implements endpoints for clip management:
//! - Create clip
//! - Get clip by owner/name
//! - List user's clips
//! - List organization's clips
//! - List public clips (explore)
//! - Update clip metadata
//! - Delete clip
//! - Fork clip
//! - Read clip files

pub mod common;
pub mod crud;
pub mod files;
pub mod fork;
pub mod list;
pub mod revisions;
pub mod stars;
pub mod types;

// Re-export all handlers and their utoipa path types
pub use crud::{
	__path_create_clip, __path_delete_clip, __path_get_clip, __path_update_clip, create_clip,
	delete_clip, get_clip, update_clip,
};
pub use files::{
	__path_get_clip_file, __path_get_clip_file_raw, __path_list_clip_files,
	__path_update_clip_files, get_clip_file, get_clip_file_raw, list_clip_files, update_clip_files,
};
pub use fork::{__path_fork_clip, fork_clip};
pub use list::{
	__path_list_org_clips, __path_list_public_clips, __path_list_user_clips, __path_search_clips,
	list_org_clips, list_public_clips, list_user_clips, search_clips,
};
pub use revisions::{__path_list_clip_revisions, list_clip_revisions};
pub use stars::{
	__path_get_clip_star_status, __path_list_starred_clips, __path_star_clip, __path_unstar_clip,
	get_clip_star_status, list_starred_clips, star_clip, unstar_clip,
};

// Re-export API types for external use
pub use common::{
	ClipFileResponse, ClipFilesResponse, ClipListResponse, ClipResponse, ClipRevisionResponse,
	ClipRevisionsResponse, ClipSearchHitResponse, ClipSearchResponse, CreateClipFile,
	CreateClipRequest, ForkClipRequest, ListClipsQuery, SearchClipsQuery, StarClipResponse,
	UpdateClipRequest, UpdateFilesRequest,
};

pub use types::ClipsErrorResponse;
