// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTP client for sessions analytics API.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use loom_common_secret::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionSummary {
	pub id: String,
	pub distinct_id: String,
	pub status: String,
	pub release: Option<String>,
	pub environment: String,
	pub error_count: u32,
	pub crash_count: u32,
	pub crashed: bool,
	pub platform: String,
	pub started_at: DateTime<Utc>,
	pub ended_at: Option<DateTime<Utc>>,
	pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListSessionsResponse {
	pub sessions: Vec<SessionSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReleaseHealth {
	pub release: String,
	pub environment: String,
	pub total_sessions: u64,
	pub crashed_sessions: u64,
	pub crash_free_session_rate: f64,
	pub crash_free_user_rate: f64,
	pub adoption_rate: f64,
	pub adoption_stage: String,
	pub first_seen: DateTime<Utc>,
	pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListReleasesHealthResponse {
	pub releases: Vec<ReleaseHealth>,
}

pub struct SessionsClient {
	base_url: Url,
	http: reqwest::Client,
	auth_token: Option<SecretString>,
}

impl SessionsClient {
	pub fn new(base_url: &str) -> Result<Self> {
		let base_url = Url::parse(base_url).context("invalid server URL")?;
		let http = loom_common_http::new_client();
		Ok(Self {
			base_url,
			http,
			auth_token: None,
		})
	}

	pub fn with_token(mut self, token: SecretString) -> Self {
		self.auth_token = Some(token);
		self
	}

	fn auth_header(&self) -> Option<String> {
		self
			.auth_token
			.as_ref()
			.map(|t| format!("Bearer {}", t.expose()))
	}

	// ========================================================================
	// Sessions
	// ========================================================================

	pub async fn list_sessions(
		&self,
		project_id: &str,
		limit: Option<u32>,
		offset: Option<u32>,
	) -> Result<Vec<SessionSummary>> {
		let mut url = self
			.base_url
			.join(&format!("api/app-sessions?project_id={}", project_id))?;

		if let Some(l) = limit {
			url.query_pairs_mut().append_pair("limit", &l.to_string());
		}
		if let Some(o) = offset {
			url.query_pairs_mut().append_pair("offset", &o.to_string());
		}

		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list sessions: {status} - {body}");
		}

		let resp: ListSessionsResponse = response.json().await?;
		Ok(resp.sessions)
	}

	// ========================================================================
	// Release Health
	// ========================================================================

	pub async fn list_release_health(
		&self,
		project_id: &str,
		environment: Option<&str>,
		days: Option<u32>,
	) -> Result<Vec<ReleaseHealth>> {
		let mut url = self.base_url.join(&format!(
			"api/app-sessions/releases?project_id={}",
			project_id
		))?;

		if let Some(env) = environment {
			url.query_pairs_mut().append_pair("environment", env);
		}
		if let Some(d) = days {
			url.query_pairs_mut().append_pair("days", &d.to_string());
		}

		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list release health: {status} - {body}");
		}

		let resp: ListReleasesHealthResponse = response.json().await?;
		Ok(resp.releases)
	}

	pub async fn get_release_health(
		&self,
		project_id: &str,
		version: &str,
		environment: Option<&str>,
		days: Option<u32>,
	) -> Result<ReleaseHealth> {
		let mut url = self.base_url.join(&format!(
			"api/app-sessions/releases/{}?project_id={}",
			version, project_id
		))?;

		if let Some(env) = environment {
			url.query_pairs_mut().append_pair("environment", env);
		}
		if let Some(d) = days {
			url.query_pairs_mut().append_pair("days", &d.to_string());
		}

		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to get release health: {status} - {body}");
		}

		let health: ReleaseHealth = response.json().await?;
		Ok(health)
	}
}
