// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for invitation routes.

use loom_server_auth::{org::OrgVisibility, Visibility};
use uuid::Uuid;

// Re-export API types
pub use loom_server_api::invitations::*;

/// Generate a cryptographically secure invitation token.
pub fn generate_invitation_token() -> String {
	format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Convert org visibility to ABAC visibility.
pub fn org_visibility_to_abac(v: OrgVisibility) -> Visibility {
	match v {
		OrgVisibility::Public => Visibility::Public,
		OrgVisibility::Unlisted => Visibility::Organization,
		OrgVisibility::Private => Visibility::Private,
	}
}
