// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

pub mod api_key;
pub mod docs;
mod error;
pub mod org;
pub mod pool;
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
pub use docs::{DocIndexEntry, DocSearchHit, DocSearchParams, DocsRepository};
pub use error::{DbError, Result};
pub use org::OrgRepository;
pub use pool::create_pool;
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
