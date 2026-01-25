// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Crash analytics HTTP handlers.
//!
//! This module implements endpoints for crash event capture, issue management,
//! project configuration, and real-time updates.
//!
//! ## Module Organization
//!
//! - [`api_keys`] - API key management (create, list, revoke)
//! - [`artifacts`] - Symbol/source map artifact management
//! - [`capture`] - Crash event ingestion endpoints
//! - [`common`] - Shared types and helpers
//! - [`events`] - Crash event queries
//! - [`issues`] - Issue management (CRUD, status changes, assignment)
//! - [`projects`] - Project CRUD operations
//! - [`releases`] - Release management
//! - [`stream`] - SSE real-time updates

pub mod api_keys;
pub mod artifacts;
pub mod capture;
pub mod common;
pub mod events;
pub mod issues;
pub mod projects;
pub mod releases;
pub mod stream;

// Re-export all public types and handlers for convenience

// Common types
pub use common::CrashErrorResponse;

// Capture types and handlers
pub use capture::{
	batch_capture_crash, capture_crash, capture_crash_with_api_key, BatchCaptureEventResult,
	BatchCaptureRequest, BatchCaptureResponse, CaptureBreadcrumb, CaptureFrame, CaptureRequest,
	CaptureResponse, CaptureStacktrace,
};
// utoipa path types for capture
pub use capture::{
	__path_batch_capture_crash, __path_capture_crash, __path_capture_crash_with_api_key,
};

// Project types and handlers
pub use projects::{
	create_project, delete_project, get_project, list_projects, update_project,
	CreateProjectRequest, ListProjectsParams, ProjectListResponse, ProjectResponse,
	UpdateProjectRequest,
};
// utoipa path types for projects
pub use projects::{
	__path_create_project, __path_delete_project, __path_get_project, __path_list_projects,
	__path_update_project,
};

// Issue types and handlers
pub use issues::{
	assign_issue, delete_issue, get_issue, ignore_issue, list_issues, resolve_issue,
	unresolve_issue, AssignIssueRequest, IssueDetailResponse, IssueMetadataResponse,
	IssueResponse, ResolveRequest,
};
// utoipa path types for issues
pub use issues::{
	__path_assign_issue, __path_delete_issue, __path_get_issue, __path_ignore_issue,
	__path_list_issues, __path_resolve_issue, __path_unresolve_issue,
};

// Event types and handlers
pub use events::{
	get_event, list_events, list_issue_events, CrashEventResponse, FrameResponse,
	ListEventsParams, StacktraceResponse,
};
// utoipa path types for events
pub use events::{__path_get_event, __path_list_events, __path_list_issue_events};

// Release types and handlers
pub use releases::{
	create_release, get_release, list_releases, CreateReleaseRequest, ReleaseResponse,
};
// utoipa path types for releases
pub use releases::{__path_create_release, __path_get_release, __path_list_releases};

// Artifact types and handlers
pub use artifacts::{
	delete_artifact, get_artifact, list_artifacts, upload_artifacts, ArtifactResponse,
	ArtifactUploadError, ListArtifactsParams, UploadArtifactResponse,
};
// utoipa path types for artifacts
pub use artifacts::{
	__path_delete_artifact, __path_get_artifact, __path_list_artifacts, __path_upload_artifacts,
};

// API key types and handlers
pub use api_keys::{
	create_api_key, list_api_keys, revoke_api_key, ApiKeyResponse, CreateApiKeyRequest,
	CreateApiKeyResponse,
};
// utoipa path types for api_keys
pub use api_keys::{__path_create_api_key, __path_list_api_keys, __path_revoke_api_key};

// Stream handler
pub use stream::stream_crash;
// utoipa path types for stream
pub use stream::__path_stream_crash;
