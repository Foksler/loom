// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::{SinkExt, StreamExt};
use loom_common_secret::SecretString;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::{
	connect_async,
	tungstenite::{client::IntoClientRequest, Message},
};
use url::Url;

#[derive(Debug, Clone, Serialize)]
pub struct CreateWeaverRequest {
	pub image: String,
	#[serde(skip_serializing_if = "HashMap::is_empty")]
	pub env: HashMap<String, String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub repo: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub branch: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub lifetime_hours: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WeaverResponse {
	pub id: String,
	pub pod_name: String,
	pub status: String,
	pub created_at: DateTime<Utc>,
	pub image: Option<String>,
	pub lifetime_hours: Option<u32>,
	pub age_hours: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListWeaversResponse {
	pub weavers: Vec<WeaverResponse>,
	#[allow(dead_code)]
	pub count: u32,
}

pub struct WeaverClient {
	base_url: Url,
	http: reqwest::Client,
	auth_token: Option<SecretString>,
}

impl WeaverClient {
	pub fn new(base_url: &str) -> Result<Self> {
		let base_url = Url::parse(base_url).context("invalid server URL")?;
		let http = loom_http::new_client();
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

	pub async fn create_weaver(&self, request: &CreateWeaverRequest) -> Result<WeaverResponse> {
		let url = self.base_url.join("api/weaver")?;
		let mut req = self.http.post(url).json(request);
		if let Some(token) = &self.auth_token {
			req = req.header("Authorization", format!("Bearer {}", token.expose()));
		}
		let response = req.send().await?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to create weaver: {status} - {body}");
		}

		let weaver: WeaverResponse = response.json().await?;
		Ok(weaver)
	}

	pub async fn list_weavers(&self) -> Result<ListWeaversResponse> {
		let url = self.base_url.join("api/weavers")?;
		let mut req = self.http.get(url);
		if let Some(token) = &self.auth_token {
			tracing::debug!("adding auth token to list_weavers request");
			req = req.header("Authorization", format!("Bearer {}", token.expose()));
		} else {
			tracing::warn!("no auth token available for list_weavers request");
		}
		let response = req.send().await?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to list weavers: {status} - {body}");
		}

		let list: ListWeaversResponse = response.json().await?;
		Ok(list)
	}

	#[allow(dead_code)]
	pub async fn get_weaver(&self, id: &str) -> Result<WeaverResponse> {
		let url = self.base_url.join(&format!("api/weaver/{id}"))?;
		let mut req = self.http.get(url);
		if let Some(token) = &self.auth_token {
			req = req.header("Authorization", format!("Bearer {}", token.expose()));
		}
		let response = req.send().await?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to get weaver: {status} - {body}");
		}

		let weaver: WeaverResponse = response.json().await?;
		Ok(weaver)
	}

	pub async fn delete_weaver(&self, id: &str) -> Result<()> {
		let url = self.base_url.join(&format!("api/weaver/{id}"))?;
		let mut req = self.http.delete(url);
		if let Some(token) = &self.auth_token {
			req = req.header("Authorization", format!("Bearer {}", token.expose()));
		}
		let response = req.send().await?;

		if !response.status().is_success() {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			anyhow::bail!("Failed to delete weaver: {status} - {body}");
		}

		Ok(())
	}

	pub fn attach_url(&self, id: &str) -> Result<Url> {
		let mut url = self.base_url.join(&format!("api/weaver/{id}/attach"))?;
		match url.scheme() {
			"http" => url.set_scheme("ws").unwrap(),
			"https" => url.set_scheme("wss").unwrap(),
			_ => {}
		}
		Ok(url)
	}

	pub async fn attach_terminal(&self, id: &str) -> Result<()> {
		let url = self.attach_url(id)?;

		tracing::info!(weaver_id = %id, url = %url, "Connecting to weaver terminal");

		let mut request = url.as_str().into_client_request()?;
		if let Some(token) = &self.auth_token {
			let auth_value = format!("Bearer {}", token.expose())
				.parse()
				.map_err(|_| anyhow::anyhow!("Invalid token format for Authorization header"))?;
			request.headers_mut().insert("Authorization", auth_value);
		}

		let (ws_stream, _) = connect_async(request)
			.await
			.context("Failed to connect to weaver")?;

		let (mut write, mut read) = ws_stream.split();

		let mut stdin = tokio::io::stdin();
		let mut stdout = tokio::io::stdout();

		let (pong_tx, mut pong_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);

		let stdin_task = tokio::spawn(async move {
			let mut buf = [0u8; 1024];
			loop {
				tokio::select! {
					result = stdin.read(&mut buf) => {
						match result {
							Ok(0) => break,
							Ok(n) => {
								if write.send(Message::Binary(buf[..n].to_vec())).await.is_err() {
									break;
								}
							}
							Err(_) => break,
						}
					}
					Some(pong_data) = pong_rx.recv() => {
						if write.send(Message::Pong(pong_data)).await.is_err() {
							break;
						}
					}
				}
			}
		});

		while let Some(msg) = read.next().await {
			match msg {
				Ok(Message::Binary(data)) => {
					stdout.write_all(&data).await?;
					stdout.flush().await?;
				}
				Ok(Message::Text(text)) => {
					eprintln!("{text}");
				}
				Ok(Message::Close(_)) => break,
				Ok(Message::Ping(data)) => {
					if pong_tx.send(data).await.is_err() {
						break;
					}
				}
				Err(e) => {
					tracing::error!(error = %e, "WebSocket error");
					return Err(anyhow::anyhow!("WebSocket error: {e}"));
				}
				_ => {}
			}
		}

		stdin_task.abort();
		Ok(())
	}
}
