// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! HTTP route handlers organized by concern.

pub mod auth;
pub mod bin;
pub mod cse;
pub mod debug;
pub mod github;
pub mod health;
pub mod threads;
pub mod weaver;

// Re-export commonly used types
pub use auth::AuthStubResponse;
pub use cse::{CseProxyRequest, CseProxyResponse, CseProxyResultItem};
pub use github::{
	GithubFileContentsRequest, GithubFileContentsResponse, GithubInstallationByRepoQuery,
	GithubRepoInfoRequest, GithubRepoInfoResponse, GithubSearchCodeRequest,
};
pub use threads::{
	ListParams, ListResponse, SearchParams, SearchResponse, SearchResponseHit,
	UpdateVisibilityRequest,
};
pub use weaver::{
	weaver_routes, CleanupApiResponse, CleanupParams, CreateWeaverApiRequest,
	ListWeaversApiResponse, ListWeaversParams, LogStreamParams, ResourceSpecApi,
	WeaverApiResponse, WeaverStatusApi,
};
