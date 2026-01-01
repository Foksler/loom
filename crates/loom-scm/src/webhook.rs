// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use loom_secret::SecretString;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::{Result, ScmError};
use crate::types::Repository;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebhookOwnerType {
	Repo,
	Org,
}

impl WebhookOwnerType {
	pub fn as_str(&self) -> &'static str {
		match self {
			WebhookOwnerType::Repo => "repo",
			WebhookOwnerType::Org => "org",
		}
	}
}

impl std::str::FromStr for WebhookOwnerType {
	type Err = ();
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"repo" => Ok(WebhookOwnerType::Repo),
			"org" => Ok(WebhookOwnerType::Org),
			_ => Err(()),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PayloadFormat {
	GitHubCompat,
	LoomV1,
}

impl PayloadFormat {
	pub fn as_str(&self) -> &'static str {
		match self {
			PayloadFormat::GitHubCompat => "github-compat",
			PayloadFormat::LoomV1 => "loom-v1",
		}
	}
}

impl std::str::FromStr for PayloadFormat {
	type Err = ();
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"github-compat" => Ok(PayloadFormat::GitHubCompat),
			"loom-v1" => Ok(PayloadFormat::LoomV1),
			_ => Err(()),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
	Pending,
	Success,
	Failed,
}

impl DeliveryStatus {
	pub fn as_str(&self) -> &'static str {
		match self {
			DeliveryStatus::Pending => "pending",
			DeliveryStatus::Success => "success",
			DeliveryStatus::Failed => "failed",
		}
	}
}

impl std::str::FromStr for DeliveryStatus {
	type Err = ();
	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
		match s {
			"pending" => Ok(DeliveryStatus::Pending),
			"success" => Ok(DeliveryStatus::Success),
			"failed" => Ok(DeliveryStatus::Failed),
			_ => Err(()),
		}
	}
}

pub struct Webhook {
	pub id: Uuid,
	pub owner_type: WebhookOwnerType,
	pub owner_id: Uuid,
	pub url: String,
	pub secret: SecretString,
	pub payload_format: PayloadFormat,
	pub events: Vec<String>,
	pub enabled: bool,
	pub created_at: DateTime<Utc>,
}

impl std::fmt::Debug for Webhook {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Webhook")
			.field("id", &self.id)
			.field("owner_type", &self.owner_type)
			.field("owner_id", &self.owner_id)
			.field("url", &self.url)
			.field("secret", &"[REDACTED]")
			.field("payload_format", &self.payload_format)
			.field("events", &self.events)
			.field("enabled", &self.enabled)
			.field("created_at", &self.created_at)
			.finish()
	}
}

impl Clone for Webhook {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			owner_type: self.owner_type,
			owner_id: self.owner_id,
			url: self.url.clone(),
			secret: SecretString::new(self.secret.expose().clone()),
			payload_format: self.payload_format,
			events: self.events.clone(),
			enabled: self.enabled,
			created_at: self.created_at,
		}
	}
}

impl Webhook {
	pub fn new(
		owner_type: WebhookOwnerType,
		owner_id: Uuid,
		url: String,
		secret: SecretString,
		payload_format: PayloadFormat,
		events: Vec<String>,
	) -> Self {
		Self {
			id: Uuid::new_v4(),
			owner_type,
			owner_id,
			url,
			secret,
			payload_format,
			events,
			enabled: true,
			created_at: Utc::now(),
		}
	}

	pub fn matches_event(&self, event: &str) -> bool {
		self.enabled && self.events.iter().any(|e| e == event)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookDelivery {
	pub id: Uuid,
	pub webhook_id: Uuid,
	pub event: String,
	pub payload: serde_json::Value,
	pub response_code: Option<i32>,
	pub response_body: Option<String>,
	pub delivered_at: Option<DateTime<Utc>>,
	pub attempts: i32,
	pub next_retry_at: Option<DateTime<Utc>>,
	pub status: DeliveryStatus,
}

impl WebhookDelivery {
	pub fn new(webhook_id: Uuid, event: String, payload: serde_json::Value) -> Self {
		Self {
			id: Uuid::new_v4(),
			webhook_id,
			event,
			payload,
			response_code: None,
			response_body: None,
			delivered_at: None,
			attempts: 0,
			next_retry_at: None,
			status: DeliveryStatus::Pending,
		}
	}
}

#[async_trait]
pub trait WebhookStore: Send + Sync {
	async fn create(&self, webhook: &Webhook) -> Result<Webhook>;
	async fn get_by_id(&self, id: Uuid) -> Result<Option<Webhook>>;
	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<Webhook>>;
	async fn list_by_org(&self, org_id: Uuid) -> Result<Vec<Webhook>>;
	async fn delete(&self, id: Uuid) -> Result<()>;
	async fn create_delivery(&self, delivery: &WebhookDelivery) -> Result<WebhookDelivery>;
	async fn update_delivery(&self, delivery: &WebhookDelivery) -> Result<()>;
	async fn get_pending_deliveries(&self) -> Result<Vec<WebhookDelivery>>;
	async fn get_webhook_for_delivery(&self, delivery_id: Uuid) -> Result<Option<Webhook>>;
}

pub struct SqliteWebhookStore {
	pool: SqlitePool,
}

impl SqliteWebhookStore {
	pub fn new(pool: SqlitePool) -> Self {
		Self { pool }
	}

	fn row_to_webhook(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Webhook> {
		let id_str: String = row.get("id");
		let owner_type_str: String = row.get("owner_type");
		let owner_id_str: String = row.get("owner_id");
		let payload_format_str: String = row.get("payload_format");
		let events_str: String = row.get("events");
		let created_at_str: String = row.get("created_at");

		Ok(Webhook {
			id: Uuid::parse_str(&id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			owner_type: owner_type_str.parse::<WebhookOwnerType>().map_err(|_| {
				ScmError::Database(sqlx::Error::Decode(
					format!("invalid owner_type: {}", owner_type_str).into(),
				))
			})?,
			owner_id: Uuid::parse_str(&owner_id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			url: row.get("url"),
			secret: SecretString::new(row.get("secret")),
			payload_format: payload_format_str.parse::<PayloadFormat>().map_err(|_| {
				ScmError::Database(sqlx::Error::Decode(
					format!("invalid payload_format: {}", payload_format_str).into(),
				))
			})?,
			events: serde_json::from_str(&events_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			enabled: row.get::<i32, _>("enabled") != 0,
			created_at: DateTime::parse_from_rfc3339(&created_at_str)
				.map(|d| d.with_timezone(&Utc))
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
		})
	}

	fn row_to_delivery(&self, row: &sqlx::sqlite::SqliteRow) -> Result<WebhookDelivery> {
		let id_str: String = row.get("id");
		let webhook_id_str: String = row.get("webhook_id");
		let payload_str: String = row.get("payload");
		let delivered_at_str: Option<String> = row.get("delivered_at");
		let next_retry_at_str: Option<String> = row.get("next_retry_at");
		let status_str: String = row.get("status");

		Ok(WebhookDelivery {
			id: Uuid::parse_str(&id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			webhook_id: Uuid::parse_str(&webhook_id_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			event: row.get("event"),
			payload: serde_json::from_str(&payload_str)
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			response_code: row.get("response_code"),
			response_body: row.get("response_body"),
			delivered_at: delivered_at_str
				.map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
				.transpose()
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			attempts: row.get("attempts"),
			next_retry_at: next_retry_at_str
				.map(|s| DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&Utc)))
				.transpose()
				.map_err(|e| ScmError::Database(sqlx::Error::Decode(e.into())))?,
			status: status_str.parse::<DeliveryStatus>().map_err(|_| {
				ScmError::Database(sqlx::Error::Decode(
					format!("invalid status: {}", status_str).into(),
				))
			})?,
		})
	}
}

#[async_trait]
impl WebhookStore for SqliteWebhookStore {
	async fn create(&self, webhook: &Webhook) -> Result<Webhook> {
		let events_json =
			serde_json::to_string(&webhook.events).map_err(|e| ScmError::GitError(e.to_string()))?;

		sqlx::query(
			r#"
			INSERT INTO webhooks (id, owner_type, owner_id, url, secret, payload_format, events, enabled, created_at)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(webhook.id.to_string())
		.bind(webhook.owner_type.as_str())
		.bind(webhook.owner_id.to_string())
		.bind(&webhook.url)
		.bind(webhook.secret.expose())
		.bind(webhook.payload_format.as_str())
		.bind(&events_json)
		.bind(webhook.enabled as i32)
		.bind(webhook.created_at.to_rfc3339())
		.execute(&self.pool)
		.await?;

		Ok(webhook.clone())
	}

	async fn get_by_id(&self, id: Uuid) -> Result<Option<Webhook>> {
		let row = sqlx::query(
			r#"
			SELECT id, owner_type, owner_id, url, secret, payload_format, events, enabled, created_at
			FROM webhooks
			WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_webhook(&r)).transpose()
	}

	async fn list_by_repo(&self, repo_id: Uuid) -> Result<Vec<Webhook>> {
		let rows = sqlx::query(
			r#"
			SELECT id, owner_type, owner_id, url, secret, payload_format, events, enabled, created_at
			FROM webhooks
			WHERE owner_type = 'repo' AND owner_id = ?
			ORDER BY created_at DESC
			"#,
		)
		.bind(repo_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_webhook(r)).collect()
	}

	async fn list_by_org(&self, org_id: Uuid) -> Result<Vec<Webhook>> {
		let rows = sqlx::query(
			r#"
			SELECT id, owner_type, owner_id, url, secret, payload_format, events, enabled, created_at
			FROM webhooks
			WHERE owner_type = 'org' AND owner_id = ?
			ORDER BY created_at DESC
			"#,
		)
		.bind(org_id.to_string())
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_webhook(r)).collect()
	}

	async fn delete(&self, id: Uuid) -> Result<()> {
		let result = sqlx::query(
			r#"
			DELETE FROM webhooks WHERE id = ?
			"#,
		)
		.bind(id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}

	async fn create_delivery(&self, delivery: &WebhookDelivery) -> Result<WebhookDelivery> {
		let payload_json = serde_json::to_string(&delivery.payload)
			.map_err(|e| ScmError::GitError(e.to_string()))?;

		sqlx::query(
			r#"
			INSERT INTO webhook_deliveries (id, webhook_id, event, payload, response_code, response_body, delivered_at, attempts, next_retry_at, status)
			VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
			"#,
		)
		.bind(delivery.id.to_string())
		.bind(delivery.webhook_id.to_string())
		.bind(&delivery.event)
		.bind(&payload_json)
		.bind(delivery.response_code)
		.bind(&delivery.response_body)
		.bind(delivery.delivered_at.map(|d| d.to_rfc3339()))
		.bind(delivery.attempts)
		.bind(delivery.next_retry_at.map(|d| d.to_rfc3339()))
		.bind(delivery.status.as_str())
		.execute(&self.pool)
		.await?;

		Ok(delivery.clone())
	}

	async fn update_delivery(&self, delivery: &WebhookDelivery) -> Result<()> {
		let result = sqlx::query(
			r#"
			UPDATE webhook_deliveries
			SET response_code = ?, response_body = ?, delivered_at = ?, attempts = ?, next_retry_at = ?, status = ?
			WHERE id = ?
			"#,
		)
		.bind(delivery.response_code)
		.bind(&delivery.response_body)
		.bind(delivery.delivered_at.map(|d| d.to_rfc3339()))
		.bind(delivery.attempts)
		.bind(delivery.next_retry_at.map(|d| d.to_rfc3339()))
		.bind(delivery.status.as_str())
		.bind(delivery.id.to_string())
		.execute(&self.pool)
		.await?;

		if result.rows_affected() == 0 {
			return Err(ScmError::NotFound);
		}

		Ok(())
	}

	async fn get_pending_deliveries(&self) -> Result<Vec<WebhookDelivery>> {
		let now = Utc::now().to_rfc3339();
		let rows = sqlx::query(
			r#"
			SELECT id, webhook_id, event, payload, response_code, response_body, delivered_at, attempts, next_retry_at, status
			FROM webhook_deliveries
			WHERE status = 'pending' AND (next_retry_at IS NULL OR next_retry_at <= ?)
			ORDER BY next_retry_at ASC, id ASC
			LIMIT 100
			"#,
		)
		.bind(&now)
		.fetch_all(&self.pool)
		.await?;

		rows.iter().map(|r| self.row_to_delivery(r)).collect()
	}

	async fn get_webhook_for_delivery(&self, delivery_id: Uuid) -> Result<Option<Webhook>> {
		let row = sqlx::query(
			r#"
			SELECT w.id, w.owner_type, w.owner_id, w.url, w.secret, w.payload_format, w.events, w.enabled, w.created_at
			FROM webhooks w
			INNER JOIN webhook_deliveries d ON d.webhook_id = w.id
			WHERE d.id = ?
			"#,
		)
		.bind(delivery_id.to_string())
		.fetch_optional(&self.pool)
		.await?;

		row.map(|r| self.row_to_webhook(&r)).transpose()
	}
}

pub mod payload {
	use super::*;

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct PushEvent {
		pub ref_name: String,
		pub before: String,
		pub after: String,
		pub pusher_name: String,
		pub pusher_email: String,
		pub commits: Vec<CommitInfo>,
	}

	#[derive(Debug, Clone, Serialize, Deserialize)]
	pub struct CommitInfo {
		pub id: String,
		pub message: String,
		pub author_name: String,
		pub author_email: String,
		pub timestamp: DateTime<Utc>,
	}

	pub fn github_push_payload(
		event: &PushEvent,
		repo: &Repository,
		base_url: &str,
		owner_name: &str,
	) -> serde_json::Value {
		let full_name = format!("{}/{}", owner_name, repo.name);
		let clone_url = format!("{}/git/{}/{}.git", base_url.trim_end_matches('/'), owner_name, repo.name);

		let commits: Vec<serde_json::Value> = event
			.commits
			.iter()
			.map(|c| {
				serde_json::json!({
					"id": c.id,
					"message": c.message,
					"timestamp": c.timestamp.to_rfc3339(),
					"author": {
						"name": c.author_name,
						"email": c.author_email
					}
				})
			})
			.collect();

		serde_json::json!({
			"ref": event.ref_name,
			"before": event.before,
			"after": event.after,
			"repository": {
				"id": repo.id.to_string(),
				"name": repo.name,
				"full_name": full_name,
				"clone_url": clone_url
			},
			"pusher": {
				"name": event.pusher_name,
				"email": event.pusher_email
			},
			"commits": commits
		})
	}

	pub fn loom_push_payload(
		event: &PushEvent,
		repo: &Repository,
		owner_name: &str,
		actor_id: Uuid,
		actor_username: &str,
	) -> serde_json::Value {
		let branch = event
			.ref_name
			.strip_prefix("refs/heads/")
			.unwrap_or(&event.ref_name);

		let commits: Vec<serde_json::Value> = event
			.commits
			.iter()
			.map(|c| {
				serde_json::json!({
					"id": c.id,
					"message": c.message,
					"author": {
						"name": c.author_name,
						"email": c.author_email
					},
					"timestamp": c.timestamp.to_rfc3339()
				})
			})
			.collect();

		serde_json::json!({
			"event": "push",
			"repo": {
				"uuid": repo.id.to_string(),
				"owner": owner_name,
				"name": repo.name
			},
			"branch": branch,
			"before": event.before,
			"after": event.after,
			"commits": commits,
			"actor": {
				"id": actor_id.to_string(),
				"username": actor_username
			}
		})
	}

	pub fn github_repo_created_payload(
		repo: &Repository,
		base_url: &str,
		owner_name: &str,
		sender_name: &str,
	) -> serde_json::Value {
		let full_name = format!("{}/{}", owner_name, repo.name);
		let clone_url = format!("{}/git/{}/{}.git", base_url.trim_end_matches('/'), owner_name, repo.name);

		serde_json::json!({
			"action": "created",
			"repository": {
				"id": repo.id.to_string(),
				"name": repo.name,
				"full_name": full_name,
				"clone_url": clone_url,
				"private": repo.visibility == crate::types::Visibility::Private,
				"created_at": repo.created_at.to_rfc3339()
			},
			"sender": {
				"login": sender_name
			}
		})
	}

	pub fn loom_repo_created_payload(
		repo: &Repository,
		owner_name: &str,
		actor_id: Uuid,
		actor_username: &str,
	) -> serde_json::Value {
		serde_json::json!({
			"event": "repo.created",
			"repo": {
				"uuid": repo.id.to_string(),
				"owner": owner_name,
				"name": repo.name,
				"visibility": repo.visibility.as_str()
			},
			"actor": {
				"id": actor_id.to_string(),
				"username": actor_username
			},
			"timestamp": Utc::now().to_rfc3339()
		})
	}

	pub fn github_repo_deleted_payload(
		repo: &Repository,
		base_url: &str,
		owner_name: &str,
		sender_name: &str,
	) -> serde_json::Value {
		let full_name = format!("{}/{}", owner_name, repo.name);
		let clone_url = format!("{}/git/{}/{}.git", base_url.trim_end_matches('/'), owner_name, repo.name);

		serde_json::json!({
			"action": "deleted",
			"repository": {
				"id": repo.id.to_string(),
				"name": repo.name,
				"full_name": full_name,
				"clone_url": clone_url
			},
			"sender": {
				"login": sender_name
			}
		})
	}

	pub fn loom_repo_deleted_payload(
		repo: &Repository,
		owner_name: &str,
		actor_id: Uuid,
		actor_username: &str,
	) -> serde_json::Value {
		serde_json::json!({
			"event": "repo.deleted",
			"repo": {
				"uuid": repo.id.to_string(),
				"owner": owner_name,
				"name": repo.name
			},
			"actor": {
				"id": actor_id.to_string(),
				"username": actor_username
			},
			"timestamp": Utc::now().to_rfc3339()
		})
	}
}

pub mod delivery {
	use super::*;
	use hmac::{Hmac, Mac};
	use sha2::Sha256;

	type HmacSha256 = Hmac<Sha256>;

	pub fn sign_payload(secret: &str, body: &[u8]) -> String {
		let mut mac =
			HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
		mac.update(body);
		let result = mac.finalize();
		let signature = hex::encode(result.into_bytes());
		format!("sha256={}", signature)
	}

	#[derive(Debug)]
	pub struct DeliveryResult {
		pub success: bool,
		pub status_code: Option<u16>,
		pub body: Option<String>,
	}

	pub async fn deliver(
		webhook: &Webhook,
		event: &str,
		payload: serde_json::Value,
		client: &reqwest::Client,
	) -> std::result::Result<DeliveryResult, reqwest::Error> {
		let body = serde_json::to_vec(&payload).unwrap_or_default();
		let signature = sign_payload(webhook.secret.expose(), &body);

		let response = client
			.post(&webhook.url)
			.header("Content-Type", "application/json")
			.header("X-Loom-Event", event)
			.header("X-Loom-Signature-256", &signature)
			.header("X-Loom-Delivery", Uuid::new_v4().to_string())
			.header("User-Agent", "Loom-Webhook/1.0")
			.body(body)
			.timeout(std::time::Duration::from_secs(30))
			.send()
			.await?;

		let status = response.status();
		let body_text = response.text().await.ok();

		Ok(DeliveryResult {
			success: status.is_success(),
			status_code: Some(status.as_u16()),
			body: body_text,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_webhook_owner_type_conversion() {
		assert_eq!(WebhookOwnerType::Repo.as_str(), "repo");
		assert_eq!(WebhookOwnerType::Org.as_str(), "org");
		assert_eq!("repo".parse::<WebhookOwnerType>(), Ok(WebhookOwnerType::Repo));
		assert_eq!("org".parse::<WebhookOwnerType>(), Ok(WebhookOwnerType::Org));
		assert!("invalid".parse::<WebhookOwnerType>().is_err());
	}

	#[test]
	fn test_payload_format_conversion() {
		assert_eq!(PayloadFormat::GitHubCompat.as_str(), "github-compat");
		assert_eq!(PayloadFormat::LoomV1.as_str(), "loom-v1");
		assert_eq!("github-compat".parse::<PayloadFormat>(), Ok(PayloadFormat::GitHubCompat));
		assert_eq!("loom-v1".parse::<PayloadFormat>(), Ok(PayloadFormat::LoomV1));
		assert!("invalid".parse::<PayloadFormat>().is_err());
	}

	#[test]
	fn test_delivery_status_conversion() {
		assert_eq!(DeliveryStatus::Pending.as_str(), "pending");
		assert_eq!(DeliveryStatus::Success.as_str(), "success");
		assert_eq!(DeliveryStatus::Failed.as_str(), "failed");
		assert_eq!("pending".parse::<DeliveryStatus>(), Ok(DeliveryStatus::Pending));
		assert_eq!("success".parse::<DeliveryStatus>(), Ok(DeliveryStatus::Success));
		assert_eq!("failed".parse::<DeliveryStatus>(), Ok(DeliveryStatus::Failed));
		assert!("invalid".parse::<DeliveryStatus>().is_err());
	}

	#[test]
	fn test_webhook_matches_event() {
		let webhook = Webhook::new(
			WebhookOwnerType::Repo,
			Uuid::new_v4(),
			"https://example.com/webhook".to_string(),
			SecretString::new("secret".to_string()),
			PayloadFormat::LoomV1,
			vec!["push".to_string(), "repo.created".to_string()],
		);

		assert!(webhook.matches_event("push"));
		assert!(webhook.matches_event("repo.created"));
		assert!(!webhook.matches_event("repo.deleted"));
	}

	#[test]
	fn test_sign_payload() {
		let signature = delivery::sign_payload("secret", b"test body");
		assert!(signature.starts_with("sha256="));
		assert_eq!(signature.len(), 71); // "sha256=" (7) + 64 hex chars
	}

	#[test]
	fn test_webhook_secret_not_in_debug() {
		let webhook = Webhook::new(
			WebhookOwnerType::Repo,
			Uuid::new_v4(),
			"https://example.com/webhook".to_string(),
			SecretString::new("super-secret-value".to_string()),
			PayloadFormat::LoomV1,
			vec!["push".to_string()],
		);
		let debug_output = format!("{:?}", webhook);
		assert!(
			!debug_output.contains("super-secret-value"),
			"Debug output should not contain the secret"
		);
		assert!(
			debug_output.contains("[REDACTED]"),
			"Debug output should contain [REDACTED]"
		);
	}

	#[test]
	fn test_sign_payload_with_secret_string() {
		let secret = SecretString::new("my-webhook-secret".to_string());
		let signature = delivery::sign_payload(secret.expose(), b"test body");
		assert!(signature.starts_with("sha256="));
		assert_eq!(signature.len(), 71);
	}
}
