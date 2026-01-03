// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Documentation search service for Loom.
//!
//! Provides FTS5 full-text search over indexed documentation.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Error)]
pub enum DocsError {
	#[error("Database error: {0}")]
	Database(#[from] sqlx::Error),

	#[error("Index file not found: {0}")]
	IndexNotFound(String),

	#[error("Invalid index format: {0}")]
	InvalidIndex(String),
}

pub type Result<T> = std::result::Result<T, DocsError>;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DocSearchHit {
	pub doc_id: String,
	pub path: String,
	pub title: String,
	pub summary: String,
	pub snippet: String,
	pub diataxis: Option<String>,
	pub tags: Option<String>,
	pub rank: f64,
}

#[derive(Debug, Clone)]
pub struct DocSearchParams {
	pub query: String,
	pub diataxis: Option<String>,
	pub limit: u32,
	pub offset: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocIndexEntry {
	pub doc_id: String,
	pub path: String,
	pub title: String,
	pub summary: String,
	pub body: String,
	pub diataxis: String,
	#[serde(default)]
	pub tags: Vec<String>,
	#[serde(default)]
	pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocsIndex {
	pub version: u32,
	pub generated_at: String,
	pub docs: Vec<DocIndexEntry>,
}

pub async fn load_docs_index(pool: &SqlitePool, index_path: &str) -> Result<usize> {
	let content = tokio::fs::read_to_string(index_path)
		.await
		.map_err(|_| DocsError::IndexNotFound(index_path.to_string()))?;

	let index: DocsIndex =
		serde_json::from_str(&content).map_err(|e| DocsError::InvalidIndex(e.to_string()))?;

	let count = index.docs.len();

	sqlx::query("DELETE FROM docs_fts")
		.execute(pool)
		.await?;

	for entry in &index.docs {
		let tags = entry.tags.join(" ");
		sqlx::query(
			r#"
            INSERT INTO docs_fts (doc_id, path, title, summary, body, diataxis, tags, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
		)
		.bind(&entry.doc_id)
		.bind(&entry.path)
		.bind(&entry.title)
		.bind(&entry.summary)
		.bind(&entry.body)
		.bind(&entry.diataxis)
		.bind(&tags)
		.bind(&entry.updated_at)
		.execute(pool)
		.await?;
	}

	tracing::info!(count = count, path = index_path, "Loaded docs search index");
	Ok(count)
}

pub async fn search_docs(pool: &SqlitePool, params: &DocSearchParams) -> Result<Vec<DocSearchHit>> {
	let query = if params.diataxis.is_some() {
		r#"
        SELECT
            doc_id,
            path,
            title,
            summary,
            snippet(docs_fts, 4, '<mark>', '</mark>', '...', 32) as snippet,
            diataxis,
            tags,
            rank
        FROM docs_fts
        WHERE docs_fts MATCH ?
          AND diataxis = ?
        ORDER BY rank
        LIMIT ? OFFSET ?
        "#
	} else {
		r#"
        SELECT
            doc_id,
            path,
            title,
            summary,
            snippet(docs_fts, 4, '<mark>', '</mark>', '...', 32) as snippet,
            diataxis,
            tags,
            rank
        FROM docs_fts
        WHERE docs_fts MATCH ?
        ORDER BY rank
        LIMIT ? OFFSET ?
        "#
	};

	let hits = if let Some(ref diataxis) = params.diataxis {
		sqlx::query_as::<_, (String, String, String, String, String, Option<String>, Option<String>, f64)>(query)
			.bind(&params.query)
			.bind(diataxis)
			.bind(params.limit)
			.bind(params.offset)
			.fetch_all(pool)
			.await?
	} else {
		sqlx::query_as::<_, (String, String, String, String, String, Option<String>, Option<String>, f64)>(query)
			.bind(&params.query)
			.bind(params.limit)
			.bind(params.offset)
			.fetch_all(pool)
			.await?
	};

	Ok(hits
		.into_iter()
		.map(|(doc_id, path, title, summary, snippet, diataxis, tags, rank)| DocSearchHit {
			doc_id,
			path,
			title,
			summary,
			snippet,
			diataxis,
			tags,
			rank,
		})
		.collect())
}
