// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Team management HTTP handlers.
//!
//! Implements team endpoints per the auth-abac-system.md specification (section 26):
//! - List teams in an organization
//! - Create/get/update/delete teams
//! - Manage team members

pub mod common;
pub mod crud;
pub mod members;

// Re-export API types
pub use common::*;

// Re-export handlers
pub use crud::{
	__path_create_team, __path_delete_team, __path_get_team, __path_list_teams, __path_update_team,
	create_team, delete_team, get_team, list_teams, update_team,
};
pub use members::{
	__path_add_team_member, __path_list_team_members, __path_remove_team_member, add_team_member,
	list_team_members, remove_team_member,
};
