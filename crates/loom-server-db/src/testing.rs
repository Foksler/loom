// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use sqlx::sqlite::SqlitePool;

pub async fn create_test_pool() -> SqlitePool {
	SqlitePool::connect(":memory:").await.unwrap()
}

pub async fn create_users_table(pool: &SqlitePool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS users (
			id TEXT PRIMARY KEY,
			display_name TEXT NOT NULL,
			username TEXT UNIQUE,
			primary_email TEXT UNIQUE,
			avatar_url TEXT,
			email_visible INTEGER DEFAULT 1,
			is_system_admin INTEGER DEFAULT 0,
			is_support INTEGER DEFAULT 0,
			is_auditor INTEGER DEFAULT 0,
			created_at TEXT NOT NULL,
			updated_at TEXT NOT NULL,
			deleted_at TEXT,
			locale TEXT DEFAULT NULL
		)
		"#,
	)
	.execute(pool)
	.await
	.unwrap();
}

pub async fn create_identities_table(pool: &SqlitePool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS identities (
			id TEXT PRIMARY KEY,
			user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
			provider TEXT NOT NULL,
			provider_user_id TEXT NOT NULL,
			email TEXT NOT NULL,
			email_verified INTEGER DEFAULT 0,
			access_token TEXT,
			refresh_token TEXT,
			token_expires_at TEXT,
			created_at TEXT NOT NULL,
			UNIQUE(provider, provider_user_id)
		)
		"#,
	)
	.execute(pool)
	.await
	.unwrap();
}

pub async fn create_job_definitions_table(pool: &SqlitePool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS job_definitions (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			description TEXT,
			job_type TEXT NOT NULL,
			interval_secs INTEGER,
			enabled INTEGER NOT NULL DEFAULT 1,
			created_at TEXT NOT NULL,
			updated_at TEXT NOT NULL
		)
		"#,
	)
	.execute(pool)
	.await
	.unwrap();
}

pub async fn create_job_runs_table(pool: &SqlitePool) {
	sqlx::query(
		r#"
		CREATE TABLE IF NOT EXISTS job_runs (
			id TEXT PRIMARY KEY,
			job_id TEXT NOT NULL REFERENCES job_definitions(id),
			status TEXT NOT NULL,
			started_at TEXT NOT NULL,
			completed_at TEXT,
			duration_ms INTEGER,
			error_message TEXT,
			retry_count INTEGER NOT NULL DEFAULT 0,
			triggered_by TEXT NOT NULL,
			metadata TEXT
		)
		"#,
	)
	.execute(pool)
	.await
	.unwrap();
}

pub async fn create_user_test_pool() -> SqlitePool {
	let pool = create_test_pool().await;
	create_users_table(&pool).await;
	create_identities_table(&pool).await;
	pool
}

pub async fn create_job_test_pool() -> SqlitePool {
	let pool = create_test_pool().await;
	create_job_definitions_table(&pool).await;
	create_job_runs_table(&pool).await;
	pool
}
