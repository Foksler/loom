// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Secrets management HTTP handlers.
//!
//! Implements user-facing secret management endpoints:
//! - Organization-scoped secrets (CRUD)
//! - Repository-scoped secrets (CRUD)
//!
//! These endpoints return metadata only, never secret values.

pub mod common;
pub mod org_secrets;
pub mod repo_secrets;

// Re-export API types
pub use common::*;

// Re-export handlers
pub use org_secrets::{
	__path_create_org_secret, __path_delete_org_secret, __path_get_org_secret,
	__path_list_org_secrets, __path_update_org_secret, create_org_secret, delete_org_secret,
	get_org_secret, list_org_secrets, update_org_secret,
};
pub use repo_secrets::{
	__path_create_repo_secret, __path_delete_repo_secret, __path_get_repo_secret,
	__path_list_repo_secrets, __path_update_repo_secret, create_repo_secret, delete_repo_secret,
	get_repo_secret, list_repo_secrets, update_repo_secret,
};
