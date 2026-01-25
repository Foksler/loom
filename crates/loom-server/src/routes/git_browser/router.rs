// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Git browser API router.

use axum::routing::get;

use super::handlers::{
	compare_refs, get_blame, get_blob, get_commit, get_raw, get_repo_by_owner_name, get_tree,
	list_branches, list_commits,
};

pub fn router() -> crate::OptionalAuthRouter {
	crate::OptionalAuthRouter::new()
		.route("/api/repos/{owner}/{name}", get(get_repo_by_owner_name))
		.route("/api/repos/{owner}/{name}/branches", get(list_branches))
		.route(
			"/api/repos/{owner}/{name}/tree/{*ref_and_path}",
			get(get_tree),
		)
		.route(
			"/api/repos/{owner}/{name}/blob/{*ref_and_path}",
			get(get_blob),
		)
		.route(
			"/api/repos/{owner}/{name}/raw/{*ref_and_path}",
			get(get_raw),
		)
		.route(
			"/api/repos/{owner}/{name}/commits/{git_ref}",
			get(list_commits),
		)
		.route("/api/repos/{owner}/{name}/commit/{sha}", get(get_commit))
		.route(
			"/api/repos/{owner}/{name}/blame/{*ref_and_path}",
			get(get_blame),
		)
		.route(
			"/api/repos/{owner}/{name}/compare/{*refs}",
			get(compare_refs),
		)
}
