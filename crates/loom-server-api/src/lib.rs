// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

pub mod admin;
pub mod api_keys;
pub mod auth;
pub mod cse;
pub mod github;
pub mod invitations;
pub mod jobs;
pub mod maintenance;
pub mod mirrors;
pub mod orgs;
pub mod protection;
pub mod repos;
pub mod secrets;
pub mod sessions;
pub mod share;
pub mod teams;
pub mod threads;
pub mod users;
pub mod weaver;
pub mod webhooks;

pub use admin::{
	AccountDetailsResponse, AccountStatus, AccountsSummary, AdminErrorResponse, AdminSuccessResponse,
	AdminUserResponse, AnthropicAccountsResponse, AnthropicOAuthCallbackQuery, AuditLogEntryResponse,
	ImpersonateRequest, ImpersonateResponse, InitiateOAuthRequest, InitiateOAuthResponse,
	ListAuditLogsParams, ListAuditLogsResponse, ListUsersParams, ListUsersResponse,
	RemoveAccountResponse, UpdateRolesRequest,
};
pub use api_keys::{
	ApiKeyErrorResponse, ApiKeyResponse, ApiKeyScopeApi, ApiKeySuccessResponse,
	ApiKeyUsageListResponse, ApiKeyUsageResponse, CreateApiKeyRequest, CreateApiKeyResponse,
	ListApiKeysResponse,
};
pub use auth::{
	AuthErrorResponse, AuthProvidersResponse, AuthSuccessResponse, CurrentUserResponse,
	DeviceCodeCompleteRequest, DeviceCodeCompleteResponse, DeviceCodePollRequest,
	DeviceCodePollResponse, DeviceCodeStartResponse, MagicLinkRequest, OAuthCallbackQuery,
	WsTokenResponse,
};
pub use cse::{CseProxyRequest, CseProxyResponse, CseProxyResultItem};
pub use github::{
	GithubFileContentsRequest, GithubFileContentsResponse, GithubInstallationByRepoQuery,
	GithubRepoInfoRequest, GithubRepoInfoResponse, GithubSearchCodeRequest,
};
pub use invitations::{
	AcceptInvitationRequest, AcceptInvitationResponse, CreateInvitationRequest,
	CreateInvitationResponse, InvitationErrorResponse, InvitationResponse, InvitationSuccessResponse,
	JoinRequestResponse, ListInvitationsResponse, ListJoinRequestsResponse,
};
pub use jobs::{
	HistoryQuery, JobHealthState, JobHistoryResponse, JobInfo, JobRunInfo, JobSuccessResponse,
	LastRunInfo, ListJobsResponse, TriggerJobResponse,
};
pub use maintenance::{
	ListMaintenanceJobsQuery, ListMaintenanceJobsResponse, MaintenanceErrorResponse,
	MaintenanceJobResponse, MaintenanceJobStatusApi, MaintenanceTaskApi, TriggerGlobalSweepRequest,
	TriggerMaintenanceRequest, TriggerMaintenanceResponse,
};
pub use mirrors::{CreateMirrorRequest, ListMirrorsResponse, MirrorResponse, SyncResponse};
pub use orgs::{
	AddOrgMemberRequest, CreateOrgRequest, ListOrgMembersResponse, ListOrgsResponse,
	OrgErrorResponse, OrgMemberResponse, OrgResponse, OrgSuccessResponse, OrgVisibilityApi,
	UpdateOrgMemberRoleRequest, UpdateOrgRequest,
};
pub use protection::{
	CreateProtectionRuleRequest, ListProtectionRulesResponse, ProtectionRuleResponse,
};
pub use repos::{
	CreateRepoRequest, GrantTeamAccessRequest, ListRepoTeamAccessResponse, ListReposResponse,
	OwnerTypeApi, RepoErrorResponse, RepoResponse, RepoRoleApi, RepoSuccessResponse,
	RepoTeamAccessResponse, UpdateRepoRequest, VisibilityApi,
};
pub use secrets::{
	CreateSecretRequest, ListSecretsResponse, SecretErrorResponse, SecretMetadataResponse,
	SecretScopeApi, SecretSuccessResponse, UpdateSecretRequest,
};
pub use sessions::{
	ListSessionsResponse, SessionErrorResponse, SessionResponse, SessionSuccessResponse,
};
pub use share::{
	CreateShareLinkRequest, CreateShareLinkResponse, ShareLinkErrorResponse,
	ShareLinkSuccessResponse, SharedThreadResponse, SupportAccessApprovalResponse,
	SupportAccessErrorResponse, SupportAccessRequestResponse, SupportAccessSuccessResponse,
};
pub use teams::{
	AddTeamMemberRequest, CreateTeamRequest, ListTeamMembersResponse, ListTeamsResponse,
	TeamErrorResponse, TeamMemberResponse, TeamResponse, TeamRoleApi, TeamSuccessResponse,
	UpdateTeamRequest,
};
pub use threads::{
	ListParams, ListResponse, SearchParams, SearchResponse, SearchResponseHit,
	UpdateVisibilityRequest,
};
pub use users::{
	AccountDeletionResponse, CurrentUserProfileResponse, IdentityResponse, ListIdentitiesResponse,
	UpdateUserProfileRequest, UserErrorResponse, UserProfileResponse, UserSuccessResponse,
};
pub use weaver::{
	CleanupApiResponse, CleanupParams, CreateWeaverApiRequest, ListWeaversApiResponse,
	ListWeaversParams, LogStreamParams, ResourceSpecApi, WeaverApiResponse, WeaverStatusApi,
};
pub use webhooks::{
	CreateWebhookRequest, ListWebhooksResponse, PayloadFormatApi, WebhookErrorResponse,
	WebhookResponse, WebhookSuccessResponse,
};
