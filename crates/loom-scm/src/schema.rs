// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use sqlx::SqlitePool;

use crate::error::Result;

pub const MIGRATIONS: &str = r#"
CREATE TABLE IF NOT EXISTS repos (
    id TEXT PRIMARY KEY NOT NULL,
    owner_type TEXT NOT NULL CHECK (owner_type IN ('user', 'org')),
    owner_id TEXT NOT NULL,
    name TEXT NOT NULL,
    visibility TEXT NOT NULL DEFAULT 'private' CHECK (visibility IN ('private', 'public')),
    default_branch TEXT NOT NULL DEFAULT 'cannon',
    deleted_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (owner_type, owner_id, name)
);

CREATE INDEX IF NOT EXISTS idx_repos_owner ON repos (owner_type, owner_id);
CREATE INDEX IF NOT EXISTS idx_repos_deleted_at ON repos (deleted_at);

CREATE TABLE IF NOT EXISTS branch_protection_rules (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL,
    block_direct_push INTEGER NOT NULL DEFAULT 1,
    block_force_push INTEGER NOT NULL DEFAULT 1,
    block_deletion INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    UNIQUE (repo_id, pattern)
);

CREATE INDEX IF NOT EXISTS idx_branch_protection_repo ON branch_protection_rules (repo_id);

CREATE TABLE IF NOT EXISTS repo_team_access (
    repo_id TEXT NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    team_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('read', 'write', 'admin')),
    PRIMARY KEY (repo_id, team_id)
);

CREATE INDEX IF NOT EXISTS idx_repo_team_access_team ON repo_team_access (team_id);

CREATE TABLE IF NOT EXISTS webhooks (
    id TEXT PRIMARY KEY NOT NULL,
    owner_type TEXT NOT NULL CHECK (owner_type IN ('repo', 'org')),
    owner_id TEXT NOT NULL,
    url TEXT NOT NULL,
    secret TEXT NOT NULL,
    payload_format TEXT NOT NULL CHECK (payload_format IN ('github-compat', 'loom-v1')),
    events TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_webhooks_owner ON webhooks (owner_type, owner_id);

CREATE TABLE IF NOT EXISTS webhook_deliveries (
    id TEXT PRIMARY KEY NOT NULL,
    webhook_id TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event TEXT NOT NULL,
    payload TEXT NOT NULL,
    response_code INTEGER,
    response_body TEXT,
    delivered_at TEXT,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_retry_at TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'success', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_webhook ON webhook_deliveries (webhook_id);
CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_status ON webhook_deliveries (status, next_retry_at);
"#;

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
	sqlx::raw_sql(MIGRATIONS).execute(pool).await?;
	Ok(())
}
