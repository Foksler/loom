// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTP client for crons monitoring API.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use loom_common_secret::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MonitorSummary {
	pub id: String,
	pub slug: String,
	pub name: String,
	pub status: String,
	pub health: String,
	pub last_checkin_at: Option<DateTime<Utc>>,
	pub next_expected_at: Option<DateTime<Utc>>,
	pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListMonitorsResponse {
	pub monitors: Vec<MonitorSummary>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Monitor {
	pub id: String,
	pub org_id: String,
	pub slug: String,
	pub name: String,
	pub description: Option<String>,
	pub status: String,
	pub health: String,
	pub schedule: MonitorSchedule,
	pub timezone: String,
	pub checkin_margin_minutes: u32,
	pub max_runtime_minutes: Option<u32>,
	pub ping_key: String,
	pub environments: Vec<String>,
	pub last_checkin_at: Option<DateTime<Utc>>,
	pub next_expected_at: Option<DateTime<Utc>>,
	pub consecutive_failures: u32,
	pub total_checkins: u64,
	pub total_failures: u64,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonitorSchedule {
	Cron { expression: String },
	Interval { minutes: u32 },
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMonitorResponse {
	pub monitor: Monitor,
	pub ping_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateMonitorRequest {
	pub org_id: String,
	pub slug: String,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	pub schedule: MonitorScheduleRequest,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub timezone: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub checkin_margin_minutes: Option<u32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub max_runtime_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MonitorScheduleRequest {
	Cron { expression: String },
	Interval { minutes: u32 },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckIn {
	pub id: String,
	pub monitor_id: String,
	pub status: String,
	pub started_at: Option<DateTime<Utc>>,
	pub finished_at: DateTime<Utc>,
	pub duration_ms: Option<u64>,
	pub environment: Option<String>,
	pub release: Option<String>,
	pub exit_code: Option<i32>,
	pub output: Option<String>,
	pub source: String,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListCheckInsResponse {
	pub checkins: Vec<CheckIn>,
}

pub struct CronsClient {
	base_url: Url,
	http: reqwest::Client,
	auth_token: Option<SecretString>,
}

impl CronsClient {
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
	// Monitors
	// ========================================================================

	pub async fn list_monitors(&self, org_id: &str) -> Result<Vec<MonitorSummary>> {
		let url = self
			.base_url
			.join(&format!("api/crons/monitors?org_id={}", org_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list monitors: {status} - {body}");
		}

		let resp: ListMonitorsResponse = response.json().await?;
		Ok(resp.monitors)
	}

	pub async fn create_monitor(
		&self,
		request: &CreateMonitorRequest,
	) -> Result<CreateMonitorResponse> {
		let url = self.base_url.join("api/crons/monitors")?;
		let mut req = self.http.post(url).json(request);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to create monitor: {status} - {body}");
		}

		let resp: CreateMonitorResponse = response.json().await?;
		Ok(resp)
	}

	pub async fn get_monitor(&self, org_id: &str, slug: &str) -> Result<Monitor> {
		let url = self
			.base_url
			.join(&format!("api/crons/monitors/{}?org_id={}", slug, org_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to get monitor: {status} - {body}");
		}

		let monitor: Monitor = response.json().await?;
		Ok(monitor)
	}

	pub async fn delete_monitor(&self, org_id: &str, slug: &str) -> Result<()> {
		let url = self
			.base_url
			.join(&format!("api/crons/monitors/{}?org_id={}", slug, org_id))?;
		let mut req = self.http.delete(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to delete monitor: {status} - {body}");
		}

		Ok(())
	}

	// ========================================================================
	// Check-ins
	// ========================================================================

	pub async fn list_checkins(
		&self,
		org_id: &str,
		slug: &str,
		limit: Option<u32>,
	) -> Result<Vec<CheckIn>> {
		let limit_param = limit.map(|l| format!("&limit={}", l)).unwrap_or_default();
		let url = self.base_url.join(&format!(
			"api/crons/monitors/{}/checkins?org_id={}{}",
			slug, org_id, limit_param
		))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list check-ins: {status} - {body}");
		}

		let resp: ListCheckInsResponse = response.json().await?;
		Ok(resp.checkins)
	}

	// ========================================================================
	// Ping
	// ========================================================================

	pub async fn ping(&self, ping_key: &str) -> Result<()> {
		let url = self.base_url.join(&format!("ping/{}", ping_key))?;
		let response = self.http.get(url).send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to ping: {status} - {body}");
		}
		Ok(())
	}

	pub async fn ping_fail(&self, ping_key: &str) -> Result<()> {
		let url = self.base_url.join(&format!("ping/{}/fail", ping_key))?;
		let response = self.http.get(url).send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to ping fail: {status} - {body}");
		}
		Ok(())
	}
}
