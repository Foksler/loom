// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use loom_common_secret::SecretString;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::person::{OrgId, PersonId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub Uuid);

impl EventId {
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}

	pub fn new_v7() -> Self {
		let uuid7_val = uuid7::uuid7();
		Self(Uuid::from_bytes(*uuid7_val.as_bytes()))
	}
}

impl Default for EventId {
	fn default() -> Self {
		Self::new_v7()
	}
}

impl std::fmt::Display for EventId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::str::FromStr for EventId {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(Uuid::parse_str(s)?))
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
	pub id: EventId,
	pub org_id: OrgId,
	pub person_id: Option<PersonId>,
	pub distinct_id: String,
	pub event_name: String,
	pub properties: serde_json::Value,
	pub timestamp: DateTime<Utc>,
	#[serde(skip_serializing)]
	pub ip_address: Option<SecretString>,
	pub user_agent: Option<String>,
	pub lib: Option<String>,
	pub lib_version: Option<String>,
	pub created_at: DateTime<Utc>,
}

impl Event {
	pub fn new(org_id: OrgId, distinct_id: String, event_name: String) -> Self {
		let now = Utc::now();
		Self {
			id: EventId::new_v7(),
			org_id,
			person_id: None,
			distinct_id,
			event_name,
			properties: serde_json::json!({}),
			timestamp: now,
			ip_address: None,
			user_agent: None,
			lib: None,
			lib_version: None,
			created_at: now,
		}
	}

	pub fn with_properties(mut self, properties: serde_json::Value) -> Self {
		self.properties = properties;
		self
	}

	pub fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
		self.timestamp = timestamp;
		self
	}

	pub fn with_person_id(mut self, person_id: PersonId) -> Self {
		self.person_id = Some(person_id);
		self
	}

	pub fn with_ip_address(mut self, ip: String) -> Self {
		self.ip_address = Some(SecretString::new(ip));
		self
	}

	pub fn with_user_agent(mut self, user_agent: String) -> Self {
		self.user_agent = Some(user_agent);
		self
	}

	pub fn with_lib(mut self, lib: String, version: String) -> Self {
		self.lib = Some(lib);
		self.lib_version = Some(version);
		self
	}

	pub fn set_property(&mut self, key: &str, value: serde_json::Value) {
		if let serde_json::Value::Object(ref mut map) = self.properties {
			map.insert(key.to_string(), value);
		}
	}
}

pub const MAX_EVENT_NAME_LENGTH: usize = 200;
pub const MAX_PROPERTIES_SIZE: usize = 1024 * 1024; // 1MB

pub fn validate_event_name(name: &str) -> bool {
	if name.is_empty() || name.len() > MAX_EVENT_NAME_LENGTH {
		return false;
	}

	let mut chars = name.chars();
	match chars.next() {
		Some(c) if c.is_ascii_lowercase() || c == '$' => {}
		_ => return false,
	}

	chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '.')
}

pub fn validate_properties_size(properties: &serde_json::Value) -> bool {
	serde_json::to_string(properties)
		.map(|s| s.len() <= MAX_PROPERTIES_SIZE)
		.unwrap_or(false)
}

pub mod special_events {
	pub const PAGEVIEW: &str = "$pageview";
	pub const PAGELEAVE: &str = "$pageleave";
	pub const IDENTIFY: &str = "$identify";
	pub const FEATURE_FLAG_CALLED: &str = "$feature_flag_called";
}

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	#[test]
	fn event_id_default_is_v7() {
		let id1 = EventId::default();
		let id2 = EventId::default();
		assert_ne!(id1, id2);
	}

	#[test]
	fn event_new() {
		let org_id = OrgId::new();
		let event = Event::new(org_id, "user_123".to_string(), "button_clicked".to_string());
		assert_eq!(event.org_id, org_id);
		assert_eq!(event.distinct_id, "user_123");
		assert_eq!(event.event_name, "button_clicked");
		assert_eq!(event.properties, serde_json::json!({}));
		assert!(event.person_id.is_none());
	}

	#[test]
	fn event_with_properties() {
		let org_id = OrgId::new();
		let event = Event::new(org_id, "user_123".to_string(), "click".to_string())
			.with_properties(serde_json::json!({"button": "submit"}));
		assert_eq!(event.properties["button"], "submit");
	}

	#[test]
	fn event_set_property() {
		let org_id = OrgId::new();
		let mut event = Event::new(org_id, "user_123".to_string(), "click".to_string());
		event.set_property("button", serde_json::json!("cancel"));
		assert_eq!(event.properties["button"], "cancel");
	}

	#[test]
	fn event_ip_address_is_secret() {
		let org_id = OrgId::new();
		let event = Event::new(org_id, "user_123".to_string(), "click".to_string())
			.with_ip_address("192.168.1.1".to_string());

		let debug_output = format!("{:?}", event);
		assert!(!debug_output.contains("192.168.1.1"));
		assert!(debug_output.contains("REDACTED"));

		assert_eq!(event.ip_address.as_ref().unwrap().expose(), "192.168.1.1");
	}

	#[test]
	fn validate_event_name_valid() {
		assert!(validate_event_name("button_clicked"));
		assert!(validate_event_name("$pageview"));
		assert!(validate_event_name("$feature_flag_called"));
		assert!(validate_event_name("checkout.completed"));
		assert!(validate_event_name("abc123"));
	}

	#[test]
	fn validate_event_name_invalid() {
		assert!(!validate_event_name(""));
		assert!(!validate_event_name("Button_Clicked")); // uppercase
		assert!(!validate_event_name("123_start")); // starts with number
		assert!(!validate_event_name("a".repeat(201).as_str()));
		assert!(!validate_event_name("event-name")); // dash not allowed
		assert!(!validate_event_name("event name")); // space not allowed
	}

	#[test]
	fn validate_properties_size_valid() {
		let small = serde_json::json!({"key": "value"});
		assert!(validate_properties_size(&small));
	}

	#[test]
	fn validate_properties_size_too_large() {
		let large_string = "x".repeat(MAX_PROPERTIES_SIZE + 1);
		let large = serde_json::json!({"key": large_string});
		assert!(!validate_properties_size(&large));
	}

	proptest! {
		#[test]
		fn event_id_is_unique(_seed: u64) {
			let id1 = EventId::new_v7();
			let id2 = EventId::new_v7();
			prop_assert_ne!(id1, id2);
		}

		#[test]
		fn event_id_roundtrip(uuid_str in "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}") {
			if let Ok(id) = uuid_str.parse::<EventId>() {
				let s = id.to_string();
				let parsed: EventId = s.parse().unwrap();
				prop_assert_eq!(id, parsed);
			}
		}

		#[test]
		fn validate_event_name_lowercase_start(name in "[a-z][a-z0-9_.]{0,50}") {
			prop_assert!(validate_event_name(&name));
		}

		#[test]
		fn validate_event_name_dollar_start(name in r"\$[a-z0-9_.]{0,50}") {
			prop_assert!(validate_event_name(&name));
		}

		#[test]
		fn validate_event_name_rejects_uppercase_start(name in "[A-Z][a-z0-9_.]{0,50}") {
			prop_assert!(!validate_event_name(&name));
		}

		#[test]
		fn validate_event_name_rejects_number_start(name in "[0-9][a-z0-9_.]{0,50}") {
			prop_assert!(!validate_event_name(&name));
		}

		#[test]
		fn validate_event_name_rejects_too_long(len in (MAX_EVENT_NAME_LENGTH + 1)..=300usize) {
			let name: String = std::iter::once('a').chain((0..len - 1).map(|_| 'a')).collect();
			prop_assert!(!validate_event_name(&name));
		}

		#[test]
		fn event_serde_roundtrip(
			distinct_id in "[a-zA-Z0-9]{1,20}",
			event_name in "[a-z][a-z0-9_]{1,20}",
		) {
			let org_id = OrgId::new();
			let event = Event::new(org_id, distinct_id.clone(), event_name.clone());

			let json = serde_json::to_string(&event).unwrap();
			let parsed: Event = serde_json::from_str(&json).unwrap();

			prop_assert_eq!(parsed.distinct_id, distinct_id);
			prop_assert_eq!(parsed.event_name, event_name);
			prop_assert_eq!(parsed.org_id, org_id);
		}
	}
}
