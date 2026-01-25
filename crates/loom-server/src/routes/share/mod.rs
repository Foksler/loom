// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Thread sharing and support access HTTP handlers.
//!
//! Implements:
//! - External read-only share links (Section 12 of auth-abac-system.md)
//! - Support access flow with 31-day expiry (Section 13 of auth-abac-system.md)

pub mod common;
pub mod share_links;
pub mod support_access;

// Re-export API types
pub use common::*;

// Re-export handlers
pub use share_links::{
	__path_create_share_link, __path_get_shared_thread, __path_revoke_share_link,
	create_share_link, get_shared_thread, revoke_share_link,
};
pub use support_access::{
	__path_approve_support_access, __path_request_support_access, __path_revoke_support_access,
	approve_support_access, request_support_access, revoke_support_access,
};
