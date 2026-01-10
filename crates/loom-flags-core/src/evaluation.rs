// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{KillSwitchId, StrategyId, VariantValue};

/// Context passed by SDK for flag evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationContext {
	pub user_id: Option<String>,
	pub org_id: Option<String>,
	pub session_id: Option<String>,
	pub environment: String,
	pub attributes: HashMap<String, serde_json::Value>,
	/// GeoIP resolved server-side from request IP
	#[serde(default)]
	pub geo: Option<GeoContext>,
}

impl EvaluationContext {
	pub fn new(environment: impl Into<String>) -> Self {
		Self {
			environment: environment.into(),
			..Default::default()
		}
	}

	pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
		self.user_id = Some(user_id.into());
		self
	}

	pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
		self.org_id = Some(org_id.into());
		self
	}

	pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
		self.session_id = Some(session_id.into());
		self
	}

	pub fn with_attribute(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
		self.attributes.insert(key.into(), value);
		self
	}

	pub fn with_geo(mut self, geo: GeoContext) -> Self {
		self.geo = Some(geo);
		self
	}
}

/// GeoIP context resolved from client IP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeoContext {
	pub country: Option<String>,
	pub region: Option<String>,
	pub city: Option<String>,
}

impl GeoContext {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_country(mut self, country: impl Into<String>) -> Self {
		self.country = Some(country.into());
		self
	}

	pub fn with_region(mut self, region: impl Into<String>) -> Self {
		self.region = Some(region.into());
		self
	}

	pub fn with_city(mut self, city: impl Into<String>) -> Self {
		self.city = Some(city.into());
		self
	}
}

/// Result of evaluating a feature flag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
	pub flag_key: String,
	pub variant: String,
	pub value: VariantValue,
	pub reason: EvaluationReason,
}

impl EvaluationResult {
	/// Creates a new evaluation result.
	pub fn new(
		flag_key: impl Into<String>,
		variant: impl Into<String>,
		value: VariantValue,
		reason: EvaluationReason,
	) -> Self {
		Self {
			flag_key: flag_key.into(),
			variant: variant.into(),
			value,
			reason,
		}
	}

	/// Creates an evaluation result for when a flag is not found.
	pub fn not_found(flag_key: impl Into<String>) -> Self {
		Self {
			flag_key: flag_key.into(),
			variant: String::new(),
			value: VariantValue::Boolean(false),
			reason: EvaluationReason::Error {
				message: "Flag not found".to_string(),
			},
		}
	}
}

/// The reason for an evaluation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum EvaluationReason {
	/// No strategy, default variant used
	Default,
	/// Strategy determined the variant
	Strategy { strategy_id: StrategyId },
	/// Kill switch forced the flag off
	KillSwitch { kill_switch_id: KillSwitchId },
	/// Prerequisite flag not met
	Prerequisite { missing_flag: String },
	/// Flag disabled in this environment
	Disabled,
	/// An error occurred during evaluation
	Error { message: String },
}

impl EvaluationReason {
	pub fn is_error(&self) -> bool {
		matches!(self, EvaluationReason::Error { .. })
	}
}

/// Bulk evaluation results for all flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkEvaluationResult {
	pub results: Vec<EvaluationResult>,
	pub evaluated_at: chrono::DateTime<chrono::Utc>,
}

impl BulkEvaluationResult {
	pub fn new(results: Vec<EvaluationResult>) -> Self {
		Self {
			results,
			evaluated_at: chrono::Utc::now(),
		}
	}

	/// Gets the result for a specific flag.
	pub fn get(&self, flag_key: &str) -> Option<&EvaluationResult> {
		self.results.iter().find(|r| r.flag_key == flag_key)
	}

	/// Gets the boolean value for a flag, returning the default if not found or not a boolean.
	pub fn get_bool(&self, flag_key: &str, default: bool) -> bool {
		self.get(flag_key)
			.and_then(|r| r.value.as_bool())
			.unwrap_or(default)
	}

	/// Gets the string value for a flag, returning the default if not found or not a string.
	pub fn get_string<'a>(&'a self, flag_key: &str, default: &'a str) -> &'a str {
		self.get(flag_key)
			.and_then(|r| r.value.as_str())
			.unwrap_or(default)
	}
}

/// Statistics for a flag's usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlagStats {
	pub flag_key: String,
	pub last_evaluated_at: Option<chrono::DateTime<chrono::Utc>>,
	pub evaluation_count_24h: u64,
	pub evaluation_count_7d: u64,
	pub evaluation_count_30d: u64,
}

/// Exposure log entry for experiment tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureLog {
	pub id: ExposureLogId,
	pub flag_key: String,
	pub environment: String,
	pub user_id: Option<String>,
	pub org_id: Option<String>,
	pub variant: String,
	pub reason: EvaluationReason,
	/// Hash of evaluation context for dedup
	pub context_hash: String,
	pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Unique identifier for an exposure log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExposureLogId(pub uuid::Uuid);

impl ExposureLogId {
	pub fn new() -> Self {
		Self(uuid::Uuid::new_v4())
	}
}

impl Default for ExposureLogId {
	fn default() -> Self {
		Self::new()
	}
}

impl std::fmt::Display for ExposureLogId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_evaluation_context_builder() {
		let ctx = EvaluationContext::new("prod")
			.with_user_id("user123")
			.with_org_id("org456")
			.with_session_id("sess789")
			.with_attribute("plan", serde_json::json!("enterprise"))
			.with_geo(GeoContext::new().with_country("US").with_region("CA"));

		assert_eq!(ctx.environment, "prod");
		assert_eq!(ctx.user_id, Some("user123".to_string()));
		assert_eq!(ctx.org_id, Some("org456".to_string()));
		assert_eq!(ctx.session_id, Some("sess789".to_string()));
		assert_eq!(ctx.attributes.get("plan"), Some(&serde_json::json!("enterprise")));
		assert!(ctx.geo.is_some());
		assert_eq!(ctx.geo.as_ref().unwrap().country, Some("US".to_string()));
	}

	#[test]
	fn test_bulk_evaluation_result() {
		let results = vec![
			EvaluationResult::new(
				"feature.enabled",
				"on",
				VariantValue::Boolean(true),
				EvaluationReason::Default,
			),
			EvaluationResult::new(
				"feature.theme",
				"dark",
				VariantValue::String("dark".to_string()),
				EvaluationReason::Default,
			),
		];

		let bulk = BulkEvaluationResult::new(results);

		assert!(bulk.get_bool("feature.enabled", false));
		assert!(!bulk.get_bool("feature.nonexistent", false));
		assert_eq!(bulk.get_string("feature.theme", "light"), "dark");
		assert_eq!(bulk.get_string("feature.nonexistent", "light"), "light");
	}

	#[test]
	fn test_evaluation_reason_is_error() {
		assert!(!EvaluationReason::Default.is_error());
		assert!(!EvaluationReason::Disabled.is_error());
		assert!(EvaluationReason::Error {
			message: "test".to_string()
		}
		.is_error());
	}
}
