// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! CSE cache extension for ThreadRepository.

use loom_server_google_cse::CseResponse;

use crate::db::ThreadRepository;
use crate::error::ServerError;

/// Extension trait for CSE cache operations.
pub trait CseCacheExt {
	fn get_cse_cache(
		&self,
		query: &str,
		max_results: u32,
	) -> impl std::future::Future<Output = Result<Option<CseResponse>, ServerError>> + Send;

	fn put_cse_cache(
		&self,
		response: &CseResponse,
		max_results: u32,
	) -> impl std::future::Future<Output = Result<(), ServerError>> + Send;
}

impl CseCacheExt for ThreadRepository {
	async fn get_cse_cache(
		&self,
		query: &str,
		max_results: u32,
	) -> Result<Option<CseResponse>, ServerError> {
		use chrono::{Duration, Utc};

		let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();

		let row: Option<(String,)> = sqlx::query_as(
			r#"
            SELECT response_json
            FROM cse_cache
            WHERE query = ?1
              AND max_results = ?2
              AND created_at >= ?3
            LIMIT 1
            "#,
		)
		.bind(ThreadRepository::normalize_cache_query(query))
		.bind(max_results as i64)
		.bind(&cutoff)
		.fetch_optional(self.pool())
		.await?;

		match row {
			Some((json,)) => {
				let response: CseResponse = serde_json::from_str(&json)?;
				tracing::debug!(
					query = %query,
					max_results = max_results,
					"cse_cache: hit"
				);
				Ok(Some(response))
			}
			None => {
				tracing::debug!(
					query = %query,
					max_results = max_results,
					"cse_cache: miss"
				);
				Ok(None)
			}
		}
	}

	async fn put_cse_cache(
		&self,
		response: &CseResponse,
		max_results: u32,
	) -> Result<(), ServerError> {
		use chrono::{Duration, Utc};

		let now = Utc::now().to_rfc3339();
		let json = serde_json::to_string(response)?;

		sqlx::query(
			r#"
            INSERT INTO cse_cache (query, max_results, response_json, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(query, max_results) DO UPDATE SET
                response_json = excluded.response_json,
                created_at    = excluded.created_at
            "#,
		)
		.bind(ThreadRepository::normalize_cache_query(&response.query))
		.bind(max_results as i64)
		.bind(&json)
		.bind(&now)
		.execute(self.pool())
		.await?;

		tracing::debug!(
			query = %response.query,
			max_results = max_results,
			"cse_cache: stored"
		);

		let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
		let result = sqlx::query(
			r#"
            DELETE FROM cse_cache
            WHERE created_at < ?1
            "#,
		)
		.bind(&cutoff)
		.execute(self.pool())
		.await?;

		if result.rows_affected() > 0 {
			tracing::debug!(
				deleted = result.rows_affected(),
				"cse_cache: cleaned up expired entries"
			);
		}

		Ok(())
	}
}
