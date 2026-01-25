// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Organization invitations and join requests HTTP handlers.
//!
//! This module handles:
//! - Organization invitations (create, list, cancel, accept, get by token)
//! - Join requests (create, list, approve, reject)

pub mod common;
pub mod invitations_crud;
pub mod join_requests;

// Re-export all handlers and their utoipa path types
pub use invitations_crud::{
	__path_accept_invitation, __path_cancel_invitation, __path_create_invitation,
	__path_get_invitation, __path_list_invitations, accept_invitation, cancel_invitation,
	create_invitation, get_invitation, list_invitations,
};
pub use join_requests::{
	__path_approve_join_request, __path_create_join_request, __path_list_join_requests,
	__path_reject_join_request, approve_join_request, create_join_request, list_join_requests,
	reject_join_request,
};

// Re-export API types for external use
pub use common::{
	AcceptInvitationRequest, AcceptInvitationResponse, CreateInvitationRequest,
	CreateInvitationResponse, InvitationErrorResponse, InvitationResponse,
	InvitationSuccessResponse, JoinRequestResponse, ListInvitationsResponse,
	ListJoinRequestsResponse,
};
