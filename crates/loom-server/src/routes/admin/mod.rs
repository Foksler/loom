// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Admin HTTP handlers.
//!
//! Implements admin endpoints per the auth-abac-system.md specification section 26:
//! - List all users (system_admin only)
//! - Update global roles
//! - User impersonation
//! - Audit log queries
//!
//! # Security
//!
//! All endpoints in this module require system administrator privileges unless
//! otherwise noted. These are highly sensitive operations that should be:
//! - Monitored via audit logs
//! - Rate-limited in production
//! - Accessed only from trusted networks
//!
//! # Authorization Matrix
//!
//! | Endpoint                | Required Role                |
//! |------------------------|------------------------------|
//! | `list_users`           | `system_admin`              |
//! | `update_user_roles`    | `system_admin`              |
//! | `start_impersonation`  | `system_admin`              |
//! | `stop_impersonation`   | authenticated (any)         |
//! | `list_audit_logs`      | `system_admin` or `auditor` |

pub mod audit;
pub mod common;
pub mod impersonation;
pub mod users;

// Re-export API types
pub use common::*;

// Re-export handlers
pub use audit::{__path_list_audit_logs, list_audit_logs};
pub use impersonation::{
	__path_get_impersonation_state, __path_start_impersonation, __path_stop_impersonation,
	get_impersonation_state, start_impersonation, stop_impersonation,
};
pub use users::{
	__path_delete_user, __path_list_users, __path_update_user_roles, delete_user, list_users,
	update_user_roles,
};
