// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{EnvironmentId, OrgId};

/// Deployment environment with its own SDK keys and flag configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
	pub id: EnvironmentId,
	pub org_id: OrgId,
	/// e.g., "dev", "staging", "prod"
	pub name: String,
	/// For UI display
	pub color: Option<String>,
	pub created_at: DateTime<Utc>,
}

impl Environment {
	/// Default environments to create for new organizations.
	pub const DEFAULT_ENVIRONMENTS: &'static [(&'static str, &'static str)] =
		&[("dev", "#10b981"), ("prod", "#ef4444")];

	/// Validates the environment name format.
	///
	/// Valid names:
	/// - Lowercase alphanumeric with underscores
	/// - 2-50 characters
	pub fn validate_name(name: &str) -> bool {
		if name.len() < 2 || name.len() > 50 {
			return false;
		}

		let mut chars = name.chars();

		// First character must be lowercase letter
		match chars.next() {
			Some(c) if c.is_ascii_lowercase() => {}
			_ => return false,
		}

		chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
	}

	/// Validates a color in hex format (e.g., "#10b981").
	pub fn validate_color(color: &str) -> bool {
		if color.len() != 7 {
			return false;
		}

		let mut chars = color.chars();
		if chars.next() != Some('#') {
			return false;
		}

		chars.all(|c| c.is_ascii_hexdigit())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_validate_name_valid() {
		assert!(Environment::validate_name("dev"));
		assert!(Environment::validate_name("prod"));
		assert!(Environment::validate_name("staging"));
		assert!(Environment::validate_name("qa"));
		assert!(Environment::validate_name("test_env"));
		assert!(Environment::validate_name("prod1"));
	}

	#[test]
	fn test_validate_name_invalid() {
		// Too short
		assert!(!Environment::validate_name("a"));
		assert!(!Environment::validate_name(""));

		// Uppercase
		assert!(!Environment::validate_name("Dev"));
		assert!(!Environment::validate_name("PROD"));

		// Invalid characters
		assert!(!Environment::validate_name("my-env"));
		assert!(!Environment::validate_name("my env"));
		assert!(!Environment::validate_name("my.env"));

		// Starts with number
		assert!(!Environment::validate_name("1env"));

		// Starts with underscore
		assert!(!Environment::validate_name("_env"));
	}

	#[test]
	fn test_validate_color_valid() {
		assert!(Environment::validate_color("#10b981"));
		assert!(Environment::validate_color("#ef4444"));
		assert!(Environment::validate_color("#FFFFFF"));
		assert!(Environment::validate_color("#000000"));
		assert!(Environment::validate_color("#abc123"));
	}

	#[test]
	fn test_validate_color_invalid() {
		// Missing #
		assert!(!Environment::validate_color("10b981"));

		// Too short
		assert!(!Environment::validate_color("#fff"));
		assert!(!Environment::validate_color("#"));

		// Too long
		assert!(!Environment::validate_color("#1234567"));

		// Invalid characters
		assert!(!Environment::validate_color("#gggggg"));
		assert!(!Environment::validate_color("#abc-23"));
	}
}
