// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Core types for the Loom feature flags system.
//!
//! This crate provides shared types for feature flags, strategies, kill switches,
//! and evaluation context. It is used by both the server-side evaluation engine
//! (`loom-server-flags`) and the client SDK (`loom-flags`).
//!
//! # Overview
//!
//! The feature flags system supports:
//! - Multi-variant flags with boolean, string, or JSON values
//! - Per-environment configuration (dev, staging, prod)
//! - Rollout strategies with percentage, attribute, and geographic targeting
//! - Kill switches for emergency feature disablement
//! - Real-time updates via SSE
//!
//! # Example
//!
//! ```
//! use loom_flags_core::{
//!     EvaluationContext, EvaluationResult, EvaluationReason,
//!     Flag, Variant, VariantValue,
//! };
//!
//! // Build evaluation context
//! let ctx = EvaluationContext::new("prod")
//!     .with_user_id("user123")
//!     .with_attribute("plan", serde_json::json!("enterprise"));
//!
//! // Evaluation results include the variant and reason
//! let result = EvaluationResult::new(
//!     "feature.new_flow",
//!     "enabled",
//!     VariantValue::Boolean(true),
//!     EvaluationReason::Default,
//! );
//! ```

pub mod environment;
pub mod error;
pub mod evaluation;
pub mod flag;
pub mod kill_switch;
pub mod sdk_key;
pub mod strategy;

pub use environment::Environment;
pub use error::{FlagsError, Result};
pub use evaluation::{
	BulkEvaluationResult, EvaluationContext, EvaluationReason, EvaluationResult, ExposureLog,
	ExposureLogId, FlagStats, GeoContext,
};
pub use flag::{
	EnvironmentId, Flag, FlagConfig, FlagConfigId, FlagId, FlagPrerequisite, OrgId, UserId, Variant,
	VariantValue,
};
pub use kill_switch::{KillSwitch, KillSwitchId};
pub use sdk_key::{SdkKey, SdkKeyId, SdkKeyType};
pub use strategy::{
	AttributeOperator, Condition, GeoField, GeoOperator, PercentageKey, Schedule, ScheduleStep,
	Strategy, StrategyId,
};

#[cfg(test)]
mod tests {
	use super::*;
	use proptest::prelude::*;

	// Property-based tests for flag key validation
	proptest! {
		#[test]
		fn flag_key_starts_with_lowercase(s in "[a-z][a-z0-9_]{2,99}") {
			// Valid keys starting with lowercase should pass
			assert!(Flag::validate_key(&s));
		}

		#[test]
		fn flag_key_rejects_uppercase(s in "[A-Z][a-z0-9_]{2,99}") {
			// Keys starting with uppercase should fail
			assert!(!Flag::validate_key(&s));
		}

		#[test]
		fn flag_key_rejects_too_short(s in "[a-z][a-z0-9_]{0,1}") {
			// Keys with 1-2 chars should fail
			assert!(!Flag::validate_key(&s));
		}

		#[test]
		fn flag_key_with_dots_valid(domain in "[a-z][a-z0-9_]{1,10}", feature in "[a-z][a-z0-9_]{1,10}") {
			let key = format!("{}.{}", domain, feature);
			assert!(Flag::validate_key(&key));
		}

		#[test]
		fn environment_name_valid(s in "[a-z][a-z0-9_]{1,49}") {
			assert!(Environment::validate_name(&s));
		}

		#[test]
		fn environment_name_rejects_dashes(s in "[a-z][a-z0-9-]{1,49}") {
			// Names with dashes should fail
			if s.contains('-') {
				assert!(!Environment::validate_name(&s));
			}
		}

		#[test]
		fn kill_switch_key_valid(s in "[a-z][a-z0-9_]{2,99}") {
			assert!(KillSwitch::validate_key(&s));
		}

		#[test]
		fn color_hex_valid(r in "[0-9a-fA-F]{2}", g in "[0-9a-fA-F]{2}", b in "[0-9a-fA-F]{2}") {
			let color = format!("#{}{}{}", r, g, b);
			assert!(Environment::validate_color(&color));
		}
	}

	// Property-based tests for attribute operators
	proptest! {
		#[test]
		fn equals_is_symmetric(a: i64, b: i64) {
			let val_a = serde_json::json!(a);
			let val_b = serde_json::json!(b);
			let result = AttributeOperator::Equals.evaluate(&val_a, &val_b);
			assert_eq!(result, a == b);
		}

		#[test]
		fn not_equals_is_negation_of_equals(a: i64, b: i64) {
			let val_a = serde_json::json!(a);
			let val_b = serde_json::json!(b);
			let eq = AttributeOperator::Equals.evaluate(&val_a, &val_b);
			let neq = AttributeOperator::NotEquals.evaluate(&val_a, &val_b);
			assert_eq!(eq, !neq);
		}

		#[test]
		fn in_contains_element(values in prop::collection::vec(1i64..100, 1..10), idx in 0usize..10) {
			if !values.is_empty() {
				let idx = idx % values.len();
				let needle = serde_json::json!(values[idx]);
				let haystack = serde_json::json!(values);
				assert!(AttributeOperator::In.evaluate(&needle, &haystack));
			}
		}

		#[test]
		fn not_in_is_negation_of_in(needle: i64, haystack in prop::collection::vec(1i64..100, 0..5)) {
			let needle_val = serde_json::json!(needle);
			let haystack_val = serde_json::json!(haystack);
			let is_in = AttributeOperator::In.evaluate(&needle_val, &haystack_val);
			let not_in = AttributeOperator::NotIn.evaluate(&needle_val, &haystack_val);
			assert_eq!(is_in, !not_in);
		}
	}

	// Property-based tests for schedule evaluation
	proptest! {
		#[test]
		fn schedule_percentage_is_monotonic(
			p1 in 0u32..=50,
			p2 in 50u32..=100,
		) {
			use chrono::{TimeZone, Utc};

			let schedule = Schedule {
				steps: vec![
					ScheduleStep {
						percentage: p1,
						start_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
					},
					ScheduleStep {
						percentage: p2,
						start_at: Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap(),
					},
				],
			};

			let early = schedule.evaluate(Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap());
			let late = schedule.evaluate(Utc.with_ymd_and_hms(2024, 2, 15, 0, 0, 0).unwrap());

			// Later schedule step should have >= percentage (assuming monotonic rollout)
			assert!(late >= early || p2 < p1);
		}
	}

	// Property-based tests for geo operators
	proptest! {
		#[test]
		fn geo_in_contains_value(
			values in prop::collection::vec("[A-Z]{2}", 1..5),
			idx in 0usize..5,
		) {
			if !values.is_empty() {
				let idx = idx % values.len();
				let needle = &values[idx];
				assert!(GeoOperator::In.evaluate(needle, &values));
			}
		}

		#[test]
		fn geo_not_in_is_negation(
			needle in "[A-Z]{2}",
			values in prop::collection::vec("[A-Z]{2}", 0..5),
		) {
			let is_in = GeoOperator::In.evaluate(&needle, &values);
			let not_in = GeoOperator::NotIn.evaluate(&needle, &values);
			assert_eq!(is_in, !not_in);
		}
	}

	// Property-based tests for SDK key parsing
	proptest! {
		#[test]
		fn sdk_key_roundtrip(
			key_type in prop_oneof![Just(SdkKeyType::ClientSide), Just(SdkKeyType::ServerSide)],
			env_name in "[a-z]{2,10}",
		) {
			let generated = SdkKey::generate_key(key_type, &env_name);
			let parsed = SdkKey::parse_key(&generated);
			assert!(parsed.is_some());
			let (parsed_type, parsed_env, _) = parsed.unwrap();
			assert_eq!(parsed_type, key_type);
			assert_eq!(parsed_env, env_name);
		}
	}

	// Property-based tests for kill switch behavior
	proptest! {
		#[test]
		fn kill_switch_affects_linked_flag_when_active(
			linked_keys in prop::collection::vec("[a-z][a-z0-9_.]{2,20}", 1..5),
			idx in 0usize..5,
		) {
			use chrono::Utc;

			if !linked_keys.is_empty() {
				let idx = idx % linked_keys.len();
				let kill_switch = KillSwitch {
					id: KillSwitchId::new(),
					org_id: None,
					key: "test_kill_switch".to_string(),
					name: "Test Kill Switch".to_string(),
					description: None,
					linked_flag_keys: linked_keys.clone(),
					is_active: true,
					activated_at: Some(Utc::now()),
					activated_by: Some(UserId::new()),
					activation_reason: Some("Testing".to_string()),
					created_at: Utc::now(),
					updated_at: Utc::now(),
				};

				// Active kill switch should affect linked flags
				assert!(kill_switch.affects_flag(&linked_keys[idx]));
			}
		}

		#[test]
		fn inactive_kill_switch_does_not_affect_flags(
			linked_keys in prop::collection::vec("[a-z][a-z0-9_.]{2,20}", 1..5),
			idx in 0usize..5,
		) {
			use chrono::Utc;

			if !linked_keys.is_empty() {
				let idx = idx % linked_keys.len();
				let kill_switch = KillSwitch {
					id: KillSwitchId::new(),
					org_id: None,
					key: "test_kill_switch".to_string(),
					name: "Test Kill Switch".to_string(),
					description: None,
					linked_flag_keys: linked_keys.clone(),
					is_active: false,
					activated_at: None,
					activated_by: None,
					activation_reason: None,
					created_at: Utc::now(),
					updated_at: Utc::now(),
				};

				// Inactive kill switch should not affect any flags
				assert!(!kill_switch.affects_flag(&linked_keys[idx]));
			}
		}

		#[test]
		fn kill_switch_does_not_affect_unlinked_flags(
			linked_keys in prop::collection::vec("[a-z][a-z0-9_.]{2,20}", 1..5),
			unlinked_key in "[a-z][a-z0-9_.]{2,20}",
		) {
			use chrono::Utc;

			// Only test if unlinked key is not in linked keys
			if !linked_keys.contains(&unlinked_key) {
				let kill_switch = KillSwitch {
					id: KillSwitchId::new(),
					org_id: None,
					key: "test_kill_switch".to_string(),
					name: "Test Kill Switch".to_string(),
					description: None,
					linked_flag_keys: linked_keys,
					is_active: true,
					activated_at: Some(Utc::now()),
					activated_by: Some(UserId::new()),
					activation_reason: Some("Testing".to_string()),
					created_at: Utc::now(),
					updated_at: Utc::now(),
				};

				// Active kill switch should not affect unlinked flags
				assert!(!kill_switch.affects_flag(&unlinked_key));
			}
		}

		#[test]
		fn activate_sets_correct_state(reason in "[a-zA-Z0-9 ]{1,100}") {
			use chrono::Utc;

			let mut kill_switch = KillSwitch {
				id: KillSwitchId::new(),
				org_id: None,
				key: "test_kill_switch".to_string(),
				name: "Test Kill Switch".to_string(),
				description: None,
				linked_flag_keys: vec!["test.flag".to_string()],
				is_active: false,
				activated_at: None,
				activated_by: None,
				activation_reason: None,
				created_at: Utc::now(),
				updated_at: Utc::now(),
			};

			let user_id = UserId::new();
			let old_updated = kill_switch.updated_at;
			kill_switch.activate(user_id, reason.clone());

			assert!(kill_switch.is_active);
			assert!(kill_switch.activated_at.is_some());
			assert_eq!(kill_switch.activated_by, Some(user_id));
			assert_eq!(kill_switch.activation_reason, Some(reason));
			assert!(kill_switch.updated_at >= old_updated);
		}

		#[test]
		fn deactivate_clears_activation_state(_seed: u64) {
			use chrono::Utc;

			let mut kill_switch = KillSwitch {
				id: KillSwitchId::new(),
				org_id: None,
				key: "test_kill_switch".to_string(),
				name: "Test Kill Switch".to_string(),
				description: None,
				linked_flag_keys: vec!["test.flag".to_string()],
				is_active: true,
				activated_at: Some(Utc::now()),
				activated_by: Some(UserId::new()),
				activation_reason: Some("Initial activation".to_string()),
				created_at: Utc::now(),
				updated_at: Utc::now(),
			};

			let old_updated = kill_switch.updated_at;
			kill_switch.deactivate();

			assert!(!kill_switch.is_active);
			assert!(kill_switch.activated_at.is_none());
			assert!(kill_switch.activated_by.is_none());
			assert!(kill_switch.activation_reason.is_none());
			assert!(kill_switch.updated_at >= old_updated);
		}
	}
}
