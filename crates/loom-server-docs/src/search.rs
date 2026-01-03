// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Documentation search using SQLite FTS5.

use anyhow::Result;
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};

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

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DocSearchHit {
    pub path: String,
    pub title: String,
    pub summary: String,
    pub diataxis: String,
    pub tags: String,
    pub snippet: String,
    pub score: f64,
}

/// Search documentation using FTS5.
///
/// Returns ranked results with highlighted snippets.
pub async fn search_docs(pool: &SqlitePool, params: &DocSearchParams) -> Result<Vec<DocSearchHit>> {
    let query = params.query.trim();
    if query.is_empty() {
        return Ok(vec![]);
    }

    // Escape the query for FTS5 - wrap in quotes for phrase search
    // and escape any internal quotes
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
        .fetch_all(pool)
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
        .fetch_all(pool)
        .await?
    };

    Ok(hits)
}
