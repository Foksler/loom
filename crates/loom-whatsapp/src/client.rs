// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! HTTP client for WhatsApp Cloud API.

use std::time::Duration;

use loom_common_secret::SecretString;
use reqwest::Client;
use tracing::{debug, info, instrument};

use crate::config::WhatsAppClientConfig;
use crate::error::{Result, WhatsAppError};
use crate::types::{SendMessageRequest, SendMessageResponse};

/// Client for WhatsApp Cloud API.
#[derive(Debug, Clone)]
pub struct WhatsAppClient {
    http: Client,
    base_url: String,
    phone_number_id: String,
    access_token: SecretString,
}

impl WhatsAppClient {
    /// Create a new WhatsApp client.
    pub fn new(config: WhatsAppClientConfig) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .user_agent("loom-whatsapp/0.1.0")
            .build()
            .map_err(WhatsAppError::Network)?;

        Ok(Self {
            http,
            base_url: config.base_url,
            phone_number_id: config.phone_number_id,
            access_token: config.access_token,
        })
    }

    /// Send a text message to a phone number.
    #[instrument(skip(self), fields(phone_number_id = %self.phone_number_id))]
    pub async fn send_text(&self, to: &str, text: &str) -> Result<SendMessageResponse> {
        debug!(to = %to, "Sending WhatsApp text message");

        let request = SendMessageRequest::text(to, text);
        self.send_message(request).await
    }

    /// Send a message using the WhatsApp Cloud API.
    #[instrument(skip(self, request), fields(phone_number_id = %self.phone_number_id))]
    pub async fn send_message(&self, request: SendMessageRequest) -> Result<SendMessageResponse> {
        let url = format!(
            "{}/{}/messages",
            self.base_url, self.phone_number_id
        );

        let response = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(self.access_token.expose())
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WhatsAppError::Timeout
                } else {
                    WhatsAppError::Network(e)
                }
            })?;

        let status = response.status();

        if status.is_success() {
            let resp: SendMessageResponse = response.json().await?;
            if let Some(msg) = resp.messages.first() {
                info!(message_id = %msg.id, to = %request.to, "WhatsApp message sent");
            }
            Ok(resp)
        } else if status.as_u16() == 401 {
            let text = response.text().await.unwrap_or_default();
            Err(WhatsAppError::Unauthorized(text))
        } else if status.as_u16() == 429 {
            Err(WhatsAppError::RateLimited)
        } else {
            let text = response.text().await.unwrap_or_default();
            Err(WhatsAppError::api_error(status.as_u16(), text))
        }
    }

    /// Get the phone number ID this client is configured for.
    pub fn phone_number_id(&self) -> &str {
        &self.phone_number_id
    }

    /// Download media from a media ID.
    /// Note: Media URLs expire in 5 minutes.
    #[instrument(skip(self))]
    pub async fn get_media_url(&self, media_id: &str) -> Result<String> {
        let url = format!("{}/{}", self.base_url, media_id);

        let response = self
            .http
            .get(&url)
            .bearer_auth(self.access_token.expose())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WhatsAppError::Timeout
                } else {
                    WhatsAppError::Network(e)
                }
            })?;

        let status = response.status();
        if status.is_success() {
            #[derive(serde::Deserialize)]
            struct MediaResponse {
                url: String,
            }
            let resp: MediaResponse = response.json().await?;
            Ok(resp.url)
        } else {
            let status_code = status.as_u16();
            let text = response.text().await.unwrap_or_default();
            Err(WhatsAppError::api_error(status_code, text))
        }
    }

    /// Download media content from a media URL.
    /// The URL must be fetched first using `get_media_url`.
    #[instrument(skip(self))]
    pub async fn download_media(&self, media_url: &str) -> Result<bytes::Bytes> {
        let response = self
            .http
            .get(media_url)
            .bearer_auth(self.access_token.expose())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    WhatsAppError::Timeout
                } else {
                    WhatsAppError::Network(e)
                }
            })?;

        let status = response.status();
        if status.is_success() {
            response.bytes().await.map_err(WhatsAppError::Network)
        } else {
            let status_code = status.as_u16();
            let text = response.text().await.unwrap_or_default();
            Err(WhatsAppError::api_error(status_code, text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = WhatsAppClientConfig::default()
            .with_phone_number_id("123456789")
            .with_access_token(SecretString::new("test_token".to_string()));

        let client = WhatsAppClient::new(config).unwrap();
        assert_eq!(client.phone_number_id(), "123456789");
    }
}
