// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Authentication scheme abstraction for Anthropic API.
//!
//! Provides a unified interface for both API key and OAuth authentication.

use loom_credentials::{CredentialError, CredentialStore};
use loom_secret::SecretString;

use super::oauth_client::OAuthClient;

/// OAuth beta header required for subscription-based authentication.
pub const OAUTH_BETA_HEADER: &str = "oauth-2025-04-20";

/// Authentication errors.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
	#[error("Missing credentials")]
	MissingCredentials,

	#[error("Credential error: {0}")]
	Credential(#[from] CredentialError),

	#[error("HTTP error: {0}")]
	Http(String),
}

/// Unified authentication for Anthropic API.
///
/// Supports both static API keys and OAuth-based subscription authentication.
#[derive(Debug)]
pub enum AnthropicAuth<S: CredentialStore> {
	/// Static pay-per-use API key.
	ApiKey { key: SecretString },

	/// OAuth-based subscription (Claude Pro/Max) or console OAuth.
	OAuth { client: OAuthClient<S> },
}

impl<S: CredentialStore> Clone for AnthropicAuth<S> {
	fn clone(&self) -> Self {
		match self {
			AnthropicAuth::ApiKey { key } => AnthropicAuth::ApiKey { key: key.clone() },
			AnthropicAuth::OAuth { client } => AnthropicAuth::OAuth {
				client: client.clone(),
			},
		}
	}
}

impl<S: CredentialStore> AnthropicAuth<S> {
	/// Create API key authentication.
	pub fn api_key(key: impl Into<String>) -> Self {
		AnthropicAuth::ApiKey {
			key: SecretString::new(key.into()),
		}
	}

	/// Check if this is API key authentication.
	pub fn is_api_key(&self) -> bool {
		matches!(self, AnthropicAuth::ApiKey { .. })
	}

	/// Check if this is OAuth authentication.
	pub fn is_oauth(&self) -> bool {
		matches!(self, AnthropicAuth::OAuth { .. })
	}

	/// Apply authentication to a request builder.
	///
	/// For API keys, sets `x-api-key` header.
	/// For OAuth, sets `Authorization: Bearer` header and `anthropic-beta` header.
	pub async fn apply_to_request(
		&self,
		request: reqwest::RequestBuilder,
	) -> Result<reqwest::RequestBuilder, AuthError> {
		match self {
			AnthropicAuth::ApiKey { key } => Ok(request.header("x-api-key", key.expose())),
			AnthropicAuth::OAuth { client } => {
				let token = client.get_access_token().await?;
				Ok(request
					.bearer_auth(token)
					.header("anthropic-beta", OAUTH_BETA_HEADER))
			}
		}
	}
}

/// Build HTTP headers for OAuth requests.
pub fn build_oauth_headers(
	access_token: &str,
	additional_beta: Option<&str>,
) -> reqwest::header::HeaderMap {
	let mut headers = reqwest::header::HeaderMap::new();

	headers.insert(
		reqwest::header::AUTHORIZATION,
		format!("Bearer {}", access_token).parse().unwrap(),
	);

	let beta_value = if let Some(additional) = additional_beta {
		format!("{},{}", OAUTH_BETA_HEADER, additional)
	} else {
		OAUTH_BETA_HEADER.to_string()
	};

	headers.insert(
		reqwest::header::HeaderName::from_static("anthropic-beta"),
		beta_value.parse().unwrap(),
	);

	headers
}

/// Build HTTP headers for API key requests.
pub fn build_api_key_headers(api_key: &str) -> reqwest::header::HeaderMap {
	let mut headers = reqwest::header::HeaderMap::new();

	headers.insert(
		reqwest::header::HeaderName::from_static("x-api-key"),
		api_key.parse().unwrap(),
	);

	headers
}

#[cfg(test)]
mod tests {
	use super::*;
	use loom_credentials::MemoryCredentialStore;
	use std::sync::Arc;

	use super::super::oauth_client::OAuthCredentials;

	#[test]
	fn test_anthropic_auth_api_key() {
		let auth: AnthropicAuth<MemoryCredentialStore> = AnthropicAuth::api_key("sk-test");
		assert!(auth.is_api_key());
		assert!(!auth.is_oauth());
	}

	#[test]
	fn test_anthropic_auth_oauth() {
		let store = Arc::new(MemoryCredentialStore::new());
		let now_ms = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_millis() as u64;

		let creds = OAuthCredentials::new(
			SecretString::new("rt_test".to_string()),
			SecretString::new("at_test".to_string()),
			now_ms + 120_000,
		);

		let auth: AnthropicAuth<MemoryCredentialStore> = AnthropicAuth::OAuth {
			client: OAuthClient::new("anthropic", creds, store),
		};

		assert!(auth.is_oauth());
		assert!(!auth.is_api_key());
	}

	#[test]
	fn test_build_oauth_headers() {
		let headers = build_oauth_headers("at_test", None);

		assert_eq!(
			headers.get(reqwest::header::AUTHORIZATION).unwrap(),
			"Bearer at_test"
		);
		assert_eq!(headers.get("anthropic-beta").unwrap(), OAUTH_BETA_HEADER);
	}

	#[test]
	fn test_build_oauth_headers_with_additional_beta() {
		let headers = build_oauth_headers("at_test", Some("max-tokens-3-5-sonnet-2024-07-15"));

		assert!(headers
			.get("anthropic-beta")
			.unwrap()
			.to_str()
			.unwrap()
			.contains(OAUTH_BETA_HEADER));
		assert!(headers
			.get("anthropic-beta")
			.unwrap()
			.to_str()
			.unwrap()
			.contains("max-tokens"));
	}

	#[test]
	fn test_build_api_key_headers() {
		let headers = build_api_key_headers("sk-test-key");

		assert_eq!(headers.get("x-api-key").unwrap(), "sk-test-key");
	}
}
