// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Documentation search repository for FTS5 operations.

use sqlx::{sqlite::SqlitePool, FromRow};

use crate::error::DbError;

#[derive(Debug, Clone, FromRow, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct DocSearchHit {
	pub path: String,
	pub title: String,
	pub summary: String,
	pub diataxis: String,
	pub tags: String,
	pub snippet: String,
	pub score: f64,
}

#[derive(Debug, Clone)]
pub struct DocSearchParams {
	pub query: String,
	pub diataxis: Option<String>,
	pub limit: u32,
	pub offset: u32,
}

impl Default for DocSearchParams {
	fn default() -> Self {
		Self {
			query: String::new(),
			diataxis: None,
			limit: 20,
			offset: 0,
		}
	}
}

#[derive(Debug, Clone)]
pub struct DocIndexEntry {
	pub doc_id: String,
	pub path: String,
	pub title: String,
	pub summary: String,
	pub body: String,
	pub diataxis: String,
	pub tags: String,
	pub updated_at: String,
}

#[derive(Clone)]
pub struct DocsRepository {
	pool: SqlitePool,
}

impl DocsRepository {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	#[tracing::instrument(skip(self))]
	pub async fn clear_docs(&self) -> Result<(), DbError> {
		sqlx::query("DELETE FROM docs_fts")
			.execute(&self.pool)
			.await?;
		tracing::debug!("cleared docs_fts table");
		Ok(())
	}

	#[tracing::instrument(skip(self, entries), fields(count = entries.len()))]
	pub async fn insert_docs(&self, entries: &[DocIndexEntry]) -> Result<(), DbError> {
		let mut tx = self.pool.begin().await?;

		sqlx::query("DELETE FROM docs_fts")
			.execute(&mut *tx)
			.await?;

		for entry in entries {
			sqlx::query(
				r#"
				INSERT INTO docs_fts (doc_id, path, title, summary, body, diataxis, tags, updated_at)
				VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
				"#,
			)
			.bind(&entry.doc_id)
			.bind(&entry.path)
			.bind(&entry.title)
			.bind(&entry.summary)
			.bind(&entry.body)
			.bind(&entry.diataxis)
			.bind(&entry.tags)
			.bind(&entry.updated_at)
			.execute(&mut *tx)
			.await?;
		}

		tx.commit().await?;
		tracing::info!(count = entries.len(), "inserted docs into search index");
		Ok(())
	}

	#[tracing::instrument(skip(self), fields(query = %params.query))]
	pub async fn search(&self, params: &DocSearchParams) -> Result<Vec<DocSearchHit>, DbError> {
		let query = params.query.trim();
		if query.is_empty() {
			return Ok(vec![]);
		}

		let escaped_query = format!("\"{}\"", query.replace('"', "\"\""));

		let hits: Vec<DocSearchHit> = if let Some(ref diataxis) = params.diataxis {
			sqlx::query_as(
				r#"
				SELECT
					path,
					title,
					COALESCE(summary, '') as summary,
					diataxis,
					COALESCE(tags, '') as tags,
					snippet(docs_fts, 4, '<mark>', '</mark>', '…', 24) AS snippet,
					bm25(docs_fts) AS score
				FROM docs_fts
				WHERE docs_fts MATCH ?1 AND diataxis = ?2
				ORDER BY score
				LIMIT ?3 OFFSET ?4
				"#,
			)
			.bind(&escaped_query)
			.bind(diataxis)
			.bind(params.limit)
			.bind(params.offset)
			.fetch_all(&self.pool)
			.await?
		} else {
			sqlx::query_as(
				r#"
				SELECT
					path,
					title,
					COALESCE(summary, '') as summary,
					diataxis,
					COALESCE(tags, '') as tags,
					snippet(docs_fts, 4, '<mark>', '</mark>', '…', 24) AS snippet,
					bm25(docs_fts) AS score
				FROM docs_fts
				WHERE docs_fts MATCH ?1
				ORDER BY score
				LIMIT ?2 OFFSET ?3
				"#,
			)
			.bind(&escaped_query)
			.bind(params.limit)
			.bind(params.offset)
			.fetch_all(&self.pool)
			.await?
		};

		tracing::debug!(count = hits.len(), "docs search completed");
		Ok(hits)
	}
}
