// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{EnvironmentId, UserId};

/// Unique identifier for an SDK key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SdkKeyId(pub Uuid);

impl SdkKeyId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for SdkKeyId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for SdkKeyId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for SdkKeyId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

/// Authentication key for SDK clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkKey {
	pub id: SdkKeyId,
	pub environment_id: EnvironmentId,
	pub key_type: SdkKeyType,
	pub name: String,
	/// Argon2 hash
	pub key_hash: String,
	pub created_by: UserId,
	pub created_at: DateTime<Utc>,
	pub last_used_at: Option<DateTime<Utc>>,
	pub revoked_at: Option<DateTime<Utc>>,
}

impl SdkKey {
	/// Prefix for client-side SDK keys.
	pub const CLIENT_PREFIX: &'static str = "loom_sdk_client_";

	/// Prefix for server-side SDK keys.
	pub const SERVER_PREFIX: &'static str = "loom_sdk_server_";

	/// Checks if this key is revoked.
	pub fn is_revoked(&self) -> bool {
		self.revoked_at.is_some()
	}

	/// Revokes this key.
	pub fn revoke(&mut self) {
		self.revoked_at = Some(Utc::now());
	}

	/// Updates the last used timestamp.
	pub fn touch(&mut self) {
		self.last_used_at = Some(Utc::now());
	}

	/// Parses an SDK key string to extract its type and environment.
	///
	/// Returns (SdkKeyType, environment_name, random_part) or None if invalid.
	pub fn parse_key(key: &str) -> Option<(SdkKeyType, String, String)> {
		if let Some(rest) = key.strip_prefix(Self::CLIENT_PREFIX) {
			let parts: Vec<&str> = rest.splitn(2, '_').collect();
			if parts.len() == 2 {
				return Some((
					SdkKeyType::ClientSide,
					parts[0].to_string(),
					parts[1].to_string(),
				));
			}
		} else if let Some(rest) = key.strip_prefix(Self::SERVER_PREFIX) {
			let parts: Vec<&str> = rest.splitn(2, '_').collect();
			if parts.len() == 2 {
				return Some((
					SdkKeyType::ServerSide,
					parts[0].to_string(),
					parts[1].to_string(),
				));
			}
		}
		None
	}

	/// Generates a new SDK key string (not the hash).
	///
	/// Format: `loom_sdk_{type}_{env}_{random}`
	pub fn generate_key(key_type: SdkKeyType, environment_name: &str) -> String {
		let random = Uuid::new_v4().to_string().replace('-', "");
		let prefix = match key_type {
			SdkKeyType::ClientSide => Self::CLIENT_PREFIX,
			SdkKeyType::ServerSide => Self::SERVER_PREFIX,
		};
		format!("{}{environment_name}_{random}", prefix)
	}
}

/// Type of SDK key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SdkKeyType {
	/// Safe for browser, single user context
	ClientSide,
	/// Secret, backend only, any user context
	ServerSide,
}

impl SdkKeyType {
	pub fn as_str(&self) -> &'static str {
		match self {
			SdkKeyType::ClientSide => "client_side",
			SdkKeyType::ServerSide => "server_side",
		}
	}
}

impl std::fmt::Display for SdkKeyType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl std::str::FromStr for SdkKeyType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"client_side" => Ok(SdkKeyType::ClientSide),
			"server_side" => Ok(SdkKeyType::ServerSide),
			_ => Err(format!("invalid SDK key type: {}", s)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_key() {
		let client_key = SdkKey::generate_key(SdkKeyType::ClientSide, "prod");
		assert!(client_key.starts_with("loom_sdk_client_prod_"));

		let server_key = SdkKey::generate_key(SdkKeyType::ServerSide, "dev");
		assert!(server_key.starts_with("loom_sdk_server_dev_"));
	}

	#[test]
	fn test_parse_key_valid() {
		let (key_type, env, random) =
			SdkKey::parse_key("loom_sdk_client_prod_abc123def456").unwrap();
		assert_eq!(key_type, SdkKeyType::ClientSide);
		assert_eq!(env, "prod");
		assert_eq!(random, "abc123def456");

		let (key_type, env, random) =
			SdkKey::parse_key("loom_sdk_server_dev_xyz789").unwrap();
		assert_eq!(key_type, SdkKeyType::ServerSide);
		assert_eq!(env, "dev");
		assert_eq!(random, "xyz789");
	}

	#[test]
	fn test_parse_key_invalid() {
		assert!(SdkKey::parse_key("invalid_key").is_none());
		assert!(SdkKey::parse_key("loom_sdk_").is_none());
		assert!(SdkKey::parse_key("loom_sdk_unknown_prod_abc").is_none());
		assert!(SdkKey::parse_key("").is_none());
	}

	#[test]
	fn test_sdk_key_type_str() {
		assert_eq!(SdkKeyType::ClientSide.as_str(), "client_side");
		assert_eq!(SdkKeyType::ServerSide.as_str(), "server_side");

		assert_eq!(
			"client_side".parse::<SdkKeyType>().unwrap(),
			SdkKeyType::ClientSide
		);
		assert_eq!(
			"server_side".parse::<SdkKeyType>().unwrap(),
			SdkKeyType::ServerSide
		);
	}
}
