// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization management HTTP handlers.
//!
//! Implements organization endpoints per the auth-abac-system.md specification:
//! - List organizations
//! - Create organization
//! - Get/update/delete organization
//! - Manage organization members

pub mod common;
pub mod join_requests;
pub mod members;
pub mod orgs_crud;
pub mod whatsapp;

use crate::impl_api_error_response;

impl_api_error_response!(loom_server_api::orgs::OrgErrorResponse);

// Re-export all handlers and their utoipa path types
pub use join_requests::{
	__path_approve_join_request, __path_create_join_request, __path_list_join_requests,
	__path_reject_join_request, approve_join_request, create_join_request, list_join_requests,
	reject_join_request,
};
pub use members::{
	__path_add_org_member, __path_list_org_members, __path_remove_org_member,
	__path_update_org_member_role, add_org_member, list_org_members, remove_org_member,
	update_org_member_role,
};
pub use orgs_crud::{
	__path_create_org, __path_delete_org, __path_get_org, __path_list_orgs, __path_restore_org,
	__path_update_org, create_org, delete_org, get_org, list_orgs, restore_org, update_org,
};

// Re-export API types (these were publicly exported by the original orgs.rs)
pub use loom_server_api::orgs::{
	AddOrgMemberRequest, CreateOrgRequest, JoinRequestResponse, ListJoinRequestsResponse,
	ListOrgMembersResponse, ListOrgsResponse, OrgErrorResponse, OrgMemberResponse, OrgResponse,
	OrgSuccessResponse, OrgVisibilityApi, UpdateOrgMemberRoleRequest, UpdateOrgRequest,
};
