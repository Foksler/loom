// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! HTTP route handlers organized by concern.

pub mod agent;
pub mod auth;
pub mod bin;
pub mod cse;
pub mod debug;
pub mod github;
pub mod health;
pub mod threads;

// Re-export commonly used types
pub use agent::{
	agent_routes, AgentApiResponse, AgentStatusApi, CleanupApiResponse, CleanupParams,
	CreateAgentApiRequest, ListAgentsApiResponse, ListAgentsParams, LogStreamParams,
	ResourceSpecApi,
};
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
