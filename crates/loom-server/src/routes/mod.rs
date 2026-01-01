// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! HTTP route handlers organized by concern.

pub mod admin;
pub mod admin_anthropic;
pub mod api_keys;
pub mod auth;
pub mod bin;
pub mod cse;
pub mod debug;
pub mod github;
pub mod health;
pub mod invitations;
pub mod orgs;
pub mod sessions;
pub mod share;
pub mod teams;
pub mod threads;
pub mod users;
pub mod weaver;

// Re-export commonly used types
pub use auth::{
	AuthErrorResponse, AuthProvidersResponse, AuthSuccessResponse, CurrentUserResponse,
	DeviceCodePollRequest, DeviceCodePollResponse, DeviceCodeStartResponse, MagicLinkRequest,
};
pub use cse::{CseProxyRequest, CseProxyResponse, CseProxyResultItem};
pub use github::{
	GithubFileContentsRequest, GithubFileContentsResponse, GithubInstallationByRepoQuery,
	GithubRepoInfoRequest, GithubRepoInfoResponse, GithubSearchCodeRequest,
};
pub use threads::{
	ListParams, ListResponse, SearchParams, SearchResponse, SearchResponseHit,
	UpdateVisibilityRequest,
};
pub use weaver::{
	weaver_routes, CleanupApiResponse, CleanupParams, CreateWeaverApiRequest,
	ListWeaversApiResponse, ListWeaversParams, LogStreamParams, ResourceSpecApi,
	WeaverApiResponse, WeaverStatusApi,
};

// Re-export session types
pub use sessions::{
	ListSessionsResponse, SessionErrorResponse, SessionResponse, SessionSuccessResponse,
};

// Re-export org types
pub use orgs::{
	AddOrgMemberRequest, CreateOrgRequest, ListOrgMembersResponse, ListOrgsResponse,
	OrgErrorResponse, OrgMemberResponse, OrgResponse, OrgSuccessResponse, OrgVisibilityApi,
	UpdateOrgRequest,
};

// Re-export user types
pub use users::{
	AccountDeletionResponse, CurrentUserProfileResponse, UpdateUserProfileRequest,
	UserErrorResponse, UserProfileResponse, UserSuccessResponse,
};

// Re-export team types
pub use teams::{
	AddTeamMemberRequest, CreateTeamRequest, ListTeamMembersResponse, ListTeamsResponse,
	TeamErrorResponse, TeamMemberResponse, TeamResponse, TeamRoleApi, TeamSuccessResponse,
	UpdateTeamRequest,
};

// Re-export API key types
pub use api_keys::{
	ApiKeyErrorResponse, ApiKeyResponse, ApiKeyScopeApi, ApiKeySuccessResponse,
	ApiKeyUsageListResponse, ApiKeyUsageResponse, CreateApiKeyRequest, CreateApiKeyResponse,
	ListApiKeysResponse,
};

// Re-export share types
pub use share::{
	CreateShareLinkRequest, CreateShareLinkResponse, ShareLinkErrorResponse,
	ShareLinkSuccessResponse, SharedThreadResponse, SupportAccessApprovalResponse,
	SupportAccessErrorResponse, SupportAccessRequestResponse, SupportAccessSuccessResponse,
};

// Re-export admin types
pub use admin::{
	AdminErrorResponse, AdminSuccessResponse, AdminUserResponse, AuditLogEntryResponse,
	ImpersonateRequest, ImpersonateResponse, ListAuditLogsResponse, ListUsersResponse,
	UpdateRolesRequest,
};

// Re-export admin_anthropic types
pub use admin_anthropic::{
	AccountDetailsResponse, AccountStatus, AccountsSummary, AnthropicAccountsResponse,
	InitiateOAuthRequest, InitiateOAuthResponse, RemoveAccountResponse,
};

// Re-export invitation types
pub use invitations::{
	AcceptInvitationRequest, AcceptInvitationResponse, CreateInvitationRequest,
	CreateInvitationResponse, InvitationErrorResponse, InvitationResponse,
	InvitationSuccessResponse, JoinRequestResponse, ListInvitationsResponse,
	ListJoinRequestsResponse,
};
