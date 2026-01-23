// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTP client for crash analytics API.

use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use loom_common_secret::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrashProject {
	pub id: String,
	pub org_id: String,
	pub name: String,
	pub slug: String,
	pub platform: String,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

// API returns array directly, not wrapped in object

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrashIssue {
	pub id: String,
	pub project_id: String,
	pub short_id: String,
	pub title: String,
	pub status: String,
	pub level: String,
	pub event_count: u64,
	pub user_count: u64,
	pub first_seen: DateTime<Utc>,
	pub last_seen: DateTime<Utc>,
	pub culprit: Option<String>,
}

// API returns array directly for issues too

#[derive(Debug, Clone, Serialize)]
pub struct CreateProjectRequest {
	pub name: String,
	pub org_id: String,
	pub platform: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct UploadArtifactResponse {
	pub id: String,
	pub name: String,
	pub artifact_type: String,
	pub size_bytes: u64,
	pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadArtifactsResponse {
	pub artifacts: Vec<UploadArtifactResponse>,
	pub count: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiKeyResponse {
	pub id: String,
	pub name: String,
	pub key_type: String,
	pub created_at: DateTime<Utc>,
	pub last_used_at: Option<DateTime<Utc>>,
	#[serde(skip_serializing)]
	pub raw_key: Option<String>,
}

// API returns array directly for API keys too

#[derive(Debug, Clone, Serialize)]
pub struct CreateApiKeyRequest {
	pub name: String,
	pub key_type: String,
}

pub struct CrashClient {
	base_url: Url,
	http: reqwest::Client,
	auth_token: Option<SecretString>,
}

impl CrashClient {
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
	// Projects
	// ========================================================================

	pub async fn list_projects(&self, org_id: &str) -> Result<Vec<CrashProject>> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects?org_id={}", org_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list projects: {status} - {body}");
		}

		let projects: Vec<CrashProject> = response.json().await?;
		Ok(projects)
	}

	pub async fn create_project(&self, request: &CreateProjectRequest) -> Result<CrashProject> {
		let url = self.base_url.join("api/crash/projects")?;
		let mut req = self.http.post(url).json(request);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to create project: {status} - {body}");
		}

		let project: CrashProject = response.json().await?;
		Ok(project)
	}

	#[allow(dead_code)]
	pub async fn get_project(&self, project_id: &str) -> Result<CrashProject> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}", project_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to get project: {status} - {body}");
		}

		let project: CrashProject = response.json().await?;
		Ok(project)
	}

	#[allow(dead_code)]
	pub async fn delete_project(&self, project_id: &str) -> Result<()> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}", project_id))?;
		let mut req = self.http.delete(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to delete project: {status} - {body}");
		}

		Ok(())
	}

	// ========================================================================
	// Issues
	// ========================================================================

	pub async fn list_issues(&self, project_id: &str) -> Result<Vec<CrashIssue>> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}/issues", project_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list issues: {status} - {body}");
		}

		let issues: Vec<CrashIssue> = response.json().await?;
		Ok(issues)
	}

	#[allow(dead_code)]
	pub async fn resolve_issue(&self, project_id: &str, issue_id: &str) -> Result<()> {
		let url = self.base_url.join(&format!(
			"api/crash/projects/{}/issues/{}/resolve",
			project_id, issue_id
		))?;
		let mut req = self.http.post(url).json(&serde_json::json!({}));
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to resolve issue: {status} - {body}");
		}

		Ok(())
	}

	// ========================================================================
	// Source Maps / Artifacts
	// ========================================================================

	pub async fn upload_sourcemaps(
		&self,
		project_id: &str,
		release: &str,
		files: &[&Path],
	) -> Result<UploadArtifactsResponse> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}/artifacts", project_id))?;

		let mut form = reqwest::multipart::Form::new().text("release", release.to_string());

		for file_path in files {
			let file_name = file_path
				.file_name()
				.and_then(|n| n.to_str())
				.unwrap_or("unknown")
				.to_string();

			let file_content = tokio::fs::read(file_path)
				.await
				.with_context(|| format!("Failed to read file: {}", file_path.display()))?;

			let part = reqwest::multipart::Part::bytes(file_content).file_name(file_name);

			form = form.part("files", part);
		}

		let mut req = self.http.post(url).multipart(form);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to upload source maps: {status} - {body}");
		}

		let result: UploadArtifactsResponse = response.json().await?;
		Ok(result)
	}

	#[allow(dead_code)]
	pub async fn list_artifacts(&self, project_id: &str) -> Result<UploadArtifactsResponse> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}/artifacts", project_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list artifacts: {status} - {body}");
		}

		let list: UploadArtifactsResponse = response.json().await?;
		Ok(list)
	}

	// ========================================================================
	// API Keys
	// ========================================================================

	pub async fn list_api_keys(&self, project_id: &str) -> Result<Vec<ApiKeyResponse>> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}/api-keys", project_id))?;
		let mut req = self.http.get(url);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list API keys: {status} - {body}");
		}

		let api_keys: Vec<ApiKeyResponse> = response.json().await?;
		Ok(api_keys)
	}

	pub async fn create_api_key(
		&self,
		project_id: &str,
		name: &str,
		key_type: &str,
	) -> Result<ApiKeyResponse> {
		let url = self
			.base_url
			.join(&format!("api/crash/projects/{}/api-keys", project_id))?;

		let request = CreateApiKeyRequest {
			name: name.to_string(),
			key_type: key_type.to_string(),
		};

		let mut req = self.http.post(url).json(&request);
		if let Some(auth) = self.auth_header() {
			req = req.header("Authorization", auth);
		}

		let response = req.send().await?;
		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to create API key: {status} - {body}");
		}

		let key: ApiKeyResponse = response.json().await?;
		Ok(key)
	}
}
