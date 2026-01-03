// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Load and import docs index into SQLite FTS5.

use anyhow::{Context, Result};
use serde::Deserialize;
use sqlx::SqlitePool;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
pub struct ExportedDoc {
    pub doc_id: String,
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub diataxis: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub updated_at: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct ExportedDocsIndex {
    pub version: u32,
    pub generated_at: String,
    pub docs: Vec<ExportedDoc>,
}

/// Load docs index from JSON file and populate FTS5 table.
///
/// This clears the existing docs_fts table and repopulates it from the JSON.
/// Called at server startup.
pub async fn load_docs_index(pool: &SqlitePool, path: &str) -> Result<usize> {
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(e) => {
            warn!("Could not read docs index at {}: {}. Docs search will be empty.", path, e);
            return Ok(0);
        }
    };

    let index: ExportedDocsIndex =
        serde_json::from_slice(&bytes).context("Failed to parse docs index JSON")?;

    info!(
        "Loading docs index: {} docs, generated at {}",
        index.docs.len(),
        index.generated_at
    );

    let mut tx = pool.begin().await?;

    // Clear existing docs
    sqlx::query("DELETE FROM docs_fts")
        .execute(&mut *tx)
        .await?;

    // Insert new docs
    for doc in &index.docs {
        let tags = doc.tags.join(" ");
        sqlx::query(
            r#"
            INSERT INTO docs_fts (doc_id, path, title, summary, body, diataxis, tags, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(&doc.doc_id)
        .bind(&doc.path)
        .bind(&doc.title)
        .bind(&doc.summary)
        .bind(&doc.body)
        .bind(&doc.diataxis)
        .bind(&tags)
        .bind(&doc.updated_at)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    info!("Loaded {} docs into search index", index.docs.len());
    Ok(index.docs.len())
}
