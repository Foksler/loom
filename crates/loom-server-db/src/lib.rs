// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

pub mod api_key;
pub mod audit;
pub mod cse;
pub mod docs;
mod error;
pub mod job;
pub mod mirror;
pub mod org;
pub mod pool;
pub mod protection;
pub mod scm;
pub mod secrets;
pub mod session;
pub mod share;
pub mod team;
pub mod thread;
pub mod types;
pub mod user;
pub mod wgtunnel;

pub use api_key::ApiKeyRepository;
pub use audit::AuditQueryRepository;
pub use cse::{normalize_cache_query, CseRepository};
pub use docs::{DocIndexEntry, DocSearchHit, DocSearchParams, DocsRepository};
pub use error::{DbError, Result};
pub use job::{JobDefinition, JobRepository, JobRun, JobStatus, TriggerSource};
pub use mirror::{
	CreateExternalMirror, CreatePushMirror, ExternalMirror, ExternalMirrorStore, MirrorBranchRule,
	MirrorRepository, Platform, PushMirror, PushMirrorStore,
};
pub use org::OrgRepository;
pub use pool::create_pool;
pub use protection::{BranchProtectionRuleRecord, ProtectionRepository, ProtectionStore};
pub use scm::{
	MaintenanceJobRecord, RepoRecord, RepoTeamAccessRecord, ScmRepository, WebhookDeliveryRecord,
	WebhookRecord,
};
pub use secrets::{
	CreateSecretParams, CreateVersionParams, EncryptedDekRow, SecretFilterParams, SecretRow,
	SecretVersionRow, SecretsRepository, StoreDekParams,
};
pub use session::SessionRepository;
pub use share::ShareRepository;
pub use team::{ScimTeam, TeamRepository};
pub use thread::{ThreadRepository, ThreadSearchHit};
pub use types::{GithubInstallation, GithubInstallationInfo, GithubRepo};
pub use user::{ScimUserRow, UserRepository};
pub use wgtunnel::WgTunnelRepository;
