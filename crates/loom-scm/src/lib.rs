// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

pub mod error;
pub mod git;
pub mod git_types;
pub mod maintenance;
pub mod protection;
pub mod repo;
pub mod schema;
pub mod types;
pub mod webhook;

pub use error::{Result, ScmError};
pub use git::GitRepository;
pub use git_types::{CommitInfo, TreeEntry, TreeEntryKind};
pub use protection::{
	check_push_allowed, matches_pattern, ProtectionStore, ProtectionViolation, PushCheck,
	SqliteProtectionStore,
};
pub use repo::{validate_repo_name, RepoStore, RepoTeamAccessStore, SqliteRepoStore, SqliteRepoTeamAccessStore};
pub use schema::{run_migrations, MIGRATIONS};
pub use types::{
	BranchProtectionRule, OwnerType, RepoRole, RepoTeamAccess, Repository, Visibility,
};
pub use webhook::{
	delivery, payload, DeliveryStatus, PayloadFormat, SqliteWebhookStore, Webhook,
	WebhookDelivery, WebhookOwnerType, WebhookStore,
};
pub use maintenance::{
	run_maintenance, run_global_sweep, run_maintenance_migrations, MaintenanceJob,
	MaintenanceJobStatus, MaintenanceJobStore, MaintenanceResult, MaintenanceTask,
	RepoMaintenanceResult, SqliteMaintenanceJobStore, MAINTENANCE_MIGRATIONS,
};
