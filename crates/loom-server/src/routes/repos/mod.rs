// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Repository management HTTP handlers.
//!
//! Implements repository endpoints per the scm-system.md specification:
//! - Create repository
//! - Get repository by ID
//! - Update repository
//! - Soft delete repository
//! - List user's repositories
//! - List organization's repositories
//! - Team access management

pub mod common;
pub mod crud;
pub mod list;
pub mod team_access;

// Re-export all handlers and their utoipa path types
pub use crud::{
	__path_create_repo, __path_delete_repo, __path_get_repo, __path_update_repo, create_repo,
	delete_repo, get_repo, update_repo,
};
pub use list::{__path_list_org_repos, __path_list_user_repos, list_org_repos, list_user_repos};
pub use team_access::{
	__path_grant_repo_team_access, __path_list_repo_team_access, __path_revoke_repo_team_access,
	grant_repo_team_access, list_repo_team_access, revoke_repo_team_access,
};

// Re-export API types for external use
pub use common::{
	CreateRepoRequest, GrantTeamAccessRequest, ListRepoTeamAccessResponse, ListReposResponse,
	RepoErrorResponse, RepoResponse, RepoSuccessResponse, RepoTeamAccessResponse, UpdateRepoRequest,
};
