// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for organization routes.

use loom_server_auth::{org::OrgVisibility, Visibility};

/// Convert OrgVisibility to ABAC Visibility.
pub fn org_visibility_to_abac(v: OrgVisibility) -> Visibility {
	match v {
		OrgVisibility::Public => Visibility::Public,
		OrgVisibility::Unlisted => Visibility::Organization,
		OrgVisibility::Private => Visibility::Private,
	}
}
