// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::person::OrgId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnalyticsApiKeyId(pub Uuid);

impl AnalyticsApiKeyId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for AnalyticsApiKeyId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for AnalyticsApiKeyId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for AnalyticsApiKeyId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for UserId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for UserId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for UserId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsKeyType {
	Write,
	ReadWrite,
}

impl AnalyticsKeyType {
	pub fn as_str(&self) -> &'static str {
		match self {
			AnalyticsKeyType::Write => "write",
			AnalyticsKeyType::ReadWrite => "read_write",
		}
	}

	pub fn can_capture(&self) -> bool {
		true
	}

	pub fn can_query(&self) -> bool {
		matches!(self, AnalyticsKeyType::ReadWrite)
	}
}

impl std::fmt::Display for AnalyticsKeyType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.as_str())
	}
}

impl std::str::FromStr for AnalyticsKeyType {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"write" => Ok(AnalyticsKeyType::Write),
			"read_write" => Ok(AnalyticsKeyType::ReadWrite),
			_ => Err(format!("invalid analytics key type: {}", s)),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsApiKey {
	pub id: AnalyticsApiKeyId,
	pub org_id: OrgId,
	pub name: String,
	pub key_type: AnalyticsKeyType,
	pub key_hash: String,
	pub created_by: UserId,
	pub created_at: DateTime<Utc>,
	pub last_used_at: Option<DateTime<Utc>>,
	pub revoked_at: Option<DateTime<Utc>>,
}

impl AnalyticsApiKey {
	pub const WRITE_PREFIX: &'static str = "loom_analytics_write_";
	pub const READ_WRITE_PREFIX: &'static str = "loom_analytics_rw_";

	pub fn is_revoked(&self) -> bool {
		self.revoked_at.is_some()
	}

	pub fn revoke(&mut self) {
		self.revoked_at = Some(Utc::now());
	}

	pub fn touch(&mut self) {
		self.last_used_at = Some(Utc::now());
	}

	pub fn parse_key(key: &str) -> Option<(AnalyticsKeyType, String)> {
		let (key_type, rest) = if let Some(rest) = key.strip_prefix(Self::WRITE_PREFIX) {
			(AnalyticsKeyType::Write, rest)
		} else if let Some(rest) = key.strip_prefix(Self::READ_WRITE_PREFIX) {
			(AnalyticsKeyType::ReadWrite, rest)
		} else {
			return None;
		};

		if rest.len() != 32 || !rest.chars().all(|c| c.is_ascii_hexdigit()) {
			return None;
		}

		Some((key_type, rest.to_string()))
	}

	pub fn generate_key(key_type: AnalyticsKeyType) -> String {
		let random = Uuid::new_v4().to_string().replace('-', "");
		let prefix = match key_type {
			AnalyticsKeyType::Write => Self::WRITE_PREFIX,
			AnalyticsKeyType::ReadWrite => Self::READ_WRITE_PREFIX,
		};
		format!("{}{}", prefix, random)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	#[test]
	fn analytics_key_type_str() {
		assert_eq!(AnalyticsKeyType::Write.as_str(), "write");
		assert_eq!(AnalyticsKeyType::ReadWrite.as_str(), "read_write");

		assert_eq!(
			"write".parse::<AnalyticsKeyType>().unwrap(),
			AnalyticsKeyType::Write
		);
		assert_eq!(
			"read_write".parse::<AnalyticsKeyType>().unwrap(),
			AnalyticsKeyType::ReadWrite
		);
	}

	#[test]
	fn analytics_key_type_permissions() {
		assert!(AnalyticsKeyType::Write.can_capture());
		assert!(!AnalyticsKeyType::Write.can_query());

		assert!(AnalyticsKeyType::ReadWrite.can_capture());
		assert!(AnalyticsKeyType::ReadWrite.can_query());
	}

	#[test]
	fn generate_key_write() {
		let key = AnalyticsApiKey::generate_key(AnalyticsKeyType::Write);
		assert!(key.starts_with("loom_analytics_write_"));
		assert_eq!(key.len(), AnalyticsApiKey::WRITE_PREFIX.len() + 32);
	}

	#[test]
	fn generate_key_read_write() {
		let key = AnalyticsApiKey::generate_key(AnalyticsKeyType::ReadWrite);
		assert!(key.starts_with("loom_analytics_rw_"));
		assert_eq!(key.len(), AnalyticsApiKey::READ_WRITE_PREFIX.len() + 32);
	}

	#[test]
	fn parse_key_valid_write() {
		let key = "loom_analytics_write_abc123def456abc123def456abc123de";
		let (key_type, random) = AnalyticsApiKey::parse_key(key).unwrap();
		assert_eq!(key_type, AnalyticsKeyType::Write);
		assert_eq!(random, "abc123def456abc123def456abc123de");
	}

	#[test]
	fn parse_key_valid_read_write() {
		let key = "loom_analytics_rw_1234567890abcdef1234567890abcdef";
		let (key_type, random) = AnalyticsApiKey::parse_key(key).unwrap();
		assert_eq!(key_type, AnalyticsKeyType::ReadWrite);
		assert_eq!(random, "1234567890abcdef1234567890abcdef");
	}

	#[test]
	fn parse_key_invalid() {
		assert!(AnalyticsApiKey::parse_key("invalid_key").is_none());
		assert!(AnalyticsApiKey::parse_key("loom_analytics_").is_none());
		assert!(AnalyticsApiKey::parse_key("").is_none());
		assert!(AnalyticsApiKey::parse_key("loom_analytics_write_abc").is_none()); // too short
		assert!(
			AnalyticsApiKey::parse_key("loom_analytics_write_abc123def456abc123def456abc123deXX")
				.is_none()
		); // too long
	}

	proptest! {
		#[test]
		fn analytics_api_key_id_is_unique(_seed: u64) {
			let id1 = AnalyticsApiKeyId::new();
			let id2 = AnalyticsApiKeyId::new();
			prop_assert_ne!(id1, id2);
		}

		#[test]
		fn analytics_api_key_id_roundtrip(uuid_str in "[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}") {
			if let Ok(id) = uuid_str.parse::<AnalyticsApiKeyId>() {
				let s = id.to_string();
				let parsed: AnalyticsApiKeyId = s.parse().unwrap();
				prop_assert_eq!(id, parsed);
			}
		}

		#[test]
		fn user_id_is_unique(_seed: u64) {
			let id1 = UserId::new();
			let id2 = UserId::new();
			prop_assert_ne!(id1, id2);
		}

		#[test]
		fn key_roundtrip_write(_seed: u64) {
			let generated = AnalyticsApiKey::generate_key(AnalyticsKeyType::Write);
			let parsed = AnalyticsApiKey::parse_key(&generated);

			prop_assert!(parsed.is_some(), "Failed to parse generated key: {}", generated);
			let (parsed_type, _random) = parsed.unwrap();
			prop_assert_eq!(parsed_type, AnalyticsKeyType::Write);
		}

		#[test]
		fn key_roundtrip_read_write(_seed: u64) {
			let generated = AnalyticsApiKey::generate_key(AnalyticsKeyType::ReadWrite);
			let parsed = AnalyticsApiKey::parse_key(&generated);

			prop_assert!(parsed.is_some(), "Failed to parse generated key: {}", generated);
			let (parsed_type, _random) = parsed.unwrap();
			prop_assert_eq!(parsed_type, AnalyticsKeyType::ReadWrite);
		}

		#[test]
		fn keys_are_unique(is_write in proptest::bool::ANY) {
			let key_type = if is_write {
				AnalyticsKeyType::Write
			} else {
				AnalyticsKeyType::ReadWrite
			};
			let key1 = AnalyticsApiKey::generate_key(key_type);
			let key2 = AnalyticsApiKey::generate_key(key_type);
			prop_assert_ne!(key1, key2, "Generated keys should be unique");
		}

		#[test]
		fn random_strings_dont_parse(garbage in "[a-zA-Z0-9_]{0,50}") {
			if !garbage.starts_with("loom_analytics_write_") && !garbage.starts_with("loom_analytics_rw_") {
				prop_assert!(AnalyticsApiKey::parse_key(&garbage).is_none());
			}
		}

		#[test]
		fn analytics_key_type_serde_roundtrip(is_write in proptest::bool::ANY) {
			let key_type = if is_write {
				AnalyticsKeyType::Write
			} else {
				AnalyticsKeyType::ReadWrite
			};

			let json = serde_json::to_string(&key_type).unwrap();
			let parsed: AnalyticsKeyType = serde_json::from_str(&json).unwrap();
			prop_assert_eq!(key_type, parsed);
		}
	}
}
