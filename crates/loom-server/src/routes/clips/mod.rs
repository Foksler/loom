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

pub mod handlers;
pub mod types;

pub use handlers::{
	__path_create_clip, __path_delete_clip, __path_fork_clip, __path_get_clip,
	__path_get_clip_file, __path_list_clip_files, __path_list_org_clips, __path_list_public_clips,
	__path_list_user_clips, __path_update_clip, create_clip, delete_clip, fork_clip, get_clip,
	get_clip_file, list_clip_files, list_org_clips, list_public_clips, list_user_clips,
	update_clip, ClipFileResponse, ClipFilesResponse, ClipListResponse, ClipResponse,
	CreateClipFile, CreateClipRequest, ForkClipRequest, ListClipsQuery, UpdateClipRequest,
};

pub use types::ClipsErrorResponse;
