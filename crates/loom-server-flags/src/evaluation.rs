// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use std::io::Cursor;

use loom_flags_core::{
	Condition, EvaluationContext, EvaluationReason, EvaluationResult, Flag, FlagConfig, GeoField,
	KillSwitch, Strategy, VariantValue,
};
use murmur3::murmur3_32;

/// Evaluates a flag for a given context.
///
/// The evaluation order is:
/// 1. Check if flag exists and is not archived
/// 2. Check environment config (enabled/disabled)
/// 3. Check kill switches (platform first, then org)
/// 4. Check prerequisites
/// 5. Evaluate strategy (conditions, percentage, schedule)
/// 6. Return default variant if no strategy or conditions not met
pub fn evaluate_flag(
	flag: &Flag,
	config: Option<&FlagConfig>,
	strategy: Option<&Strategy>,
	active_kill_switches: &[KillSwitch],
	prerequisite_results: &[(&str, &str)], // (flag_key, variant)
	context: &EvaluationContext,
) -> EvaluationResult {
	// Check if flag is archived
	if flag.is_archived() {
		return EvaluationResult::new(
			&flag.key,
			&flag.default_variant,
			get_default_value(flag),
			EvaluationReason::Error {
				message: "Flag is archived".to_string(),
			},
		);
	}

	// Check environment config
	let config = match config {
		Some(c) => c,
		None => {
			return EvaluationResult::new(
				&flag.key,
				&flag.default_variant,
				get_default_value(flag),
				EvaluationReason::Error {
					message: "No configuration for this environment".to_string(),
				},
			);
		}
	};

	if !config.enabled {
		return EvaluationResult::new(
			&flag.key,
			&flag.default_variant,
			get_default_value(flag),
			EvaluationReason::Disabled,
		);
	}

	// Check kill switches
	for kill_switch in active_kill_switches {
		if kill_switch.affects_flag(&flag.key) {
			return EvaluationResult::new(
				&flag.key,
				&flag.default_variant,
				get_default_value(flag),
				EvaluationReason::KillSwitch {
					kill_switch_id: kill_switch.id,
				},
			);
		}
	}

	// Check prerequisites
	for prereq in &flag.prerequisites {
		let matching = prerequisite_results
			.iter()
			.find(|(key, _)| *key == prereq.flag_key);

		match matching {
			Some((_, variant)) if *variant == prereq.required_variant => {
				// Prerequisite met
			}
			_ => {
				return EvaluationResult::new(
					&flag.key,
					&flag.default_variant,
					get_default_value(flag),
					EvaluationReason::Prerequisite {
						missing_flag: prereq.flag_key.clone(),
					},
				);
			}
		}
	}

	// Evaluate strategy
	if let Some(strategy) = strategy {
		// Check conditions
		if !evaluate_conditions(&strategy.conditions, context) {
			return EvaluationResult::new(
				&flag.key,
				&flag.default_variant,
				get_default_value(flag),
				EvaluationReason::Default,
			);
		}

		// Get the effective percentage (from schedule or direct)
		let percentage = match &strategy.schedule {
			Some(schedule) => schedule.evaluate(chrono::Utc::now()),
			None => strategy.percentage.unwrap_or(100),
		};

		// Check percentage
		if percentage < 100 {
			let key_value = strategy.percentage_key.get_value(
				context.user_id.as_deref(),
				context.org_id.as_deref(),
				context.session_id.as_deref(),
				&context.attributes,
			);

			if let Some(key) = key_value {
				if !evaluate_percentage(key, &flag.key, percentage) {
					return EvaluationResult::new(
						&flag.key,
						&flag.default_variant,
						get_default_value(flag),
						EvaluationReason::Default,
					);
				}
			}
		}

		// Strategy matched - return the first non-default variant or use weights
		if let Some(variant) = select_variant(flag, context) {
			return EvaluationResult::new(
				&flag.key,
				&variant.name,
				variant.value.clone(),
				EvaluationReason::Strategy {
					strategy_id: strategy.id,
				},
			);
		}
	}

	// Return default variant
	EvaluationResult::new(
		&flag.key,
		&flag.default_variant,
		get_default_value(flag),
		EvaluationReason::Default,
	)
}

/// Evaluates all conditions (AND logic).
fn evaluate_conditions(conditions: &[Condition], context: &EvaluationContext) -> bool {
	conditions.iter().all(|cond| evaluate_condition(cond, context))
}

/// Evaluates a single condition.
fn evaluate_condition(condition: &Condition, context: &EvaluationContext) -> bool {
	match condition {
		Condition::Attribute {
			attribute,
			operator,
			value,
		} => {
			let actual = context.attributes.get(attribute);
			match actual {
				Some(actual) => operator.evaluate(actual, value),
				None => false,
			}
		}
		Condition::Geographic {
			field,
			operator,
			values,
		} => {
			let geo = match &context.geo {
				Some(g) => g,
				None => return false,
			};

			let actual = match field {
				GeoField::Country => geo.country.as_deref(),
				GeoField::Region => geo.region.as_deref(),
				GeoField::City => geo.city.as_deref(),
			};

			match actual {
				Some(actual) => operator.evaluate(actual, values),
				None => false,
			}
		}
		Condition::Environment { environments } => {
			environments.iter().any(|e| e == &context.environment)
		}
	}
}

/// Evaluates percentage-based targeting using consistent hashing.
fn evaluate_percentage(key: &str, flag_key: &str, percentage: u32) -> bool {
	let input = format!("{}.{}", flag_key, key);
	let hash = murmur3_32(&mut Cursor::new(input.as_bytes()), 0).unwrap_or(0);
	let bucket = hash % 100;
	bucket < percentage
}

/// Selects a variant based on weights (for multi-variant experiments).
fn select_variant<'a>(
	flag: &'a Flag,
	context: &EvaluationContext,
) -> Option<&'a loom_flags_core::Variant> {
	// If there's only one non-default variant, use it
	let non_default: Vec<_> = flag
		.variants
		.iter()
		.filter(|v| v.name != flag.default_variant)
		.collect();

	if non_default.len() == 1 {
		return Some(non_default[0]);
	}

	// Use weighted selection based on user/org/session ID
	let key = context
		.user_id
		.as_deref()
		.or(context.org_id.as_deref())
		.or(context.session_id.as_deref())?;

	let input = format!("{}.{}.variant", flag.key, key);
	let hash = murmur3_32(&mut Cursor::new(input.as_bytes()), 0).unwrap_or(0);

	// Calculate total weight
	let total_weight: u32 = flag.variants.iter().map(|v| v.weight).sum();
	if total_weight == 0 {
		return flag.get_default_variant();
	}

	// Select variant based on hash
	let bucket = hash % total_weight;
	let mut cumulative = 0u32;

	for variant in &flag.variants {
		cumulative += variant.weight;
		if bucket < cumulative {
			return Some(variant);
		}
	}

	flag.get_default_variant()
}

/// Gets the default variant's value.
fn get_default_value(flag: &Flag) -> VariantValue {
	flag.get_default_variant()
		.map(|v| v.value.clone())
		.unwrap_or(VariantValue::Boolean(false))
}

#[cfg(test)]
mod tests {
	use super::*;
	use chrono::Utc;
	use loom_flags_core::{
		AttributeOperator, EnvironmentId, FlagConfigId, FlagId, GeoContext, GeoOperator, OrgId,
		PercentageKey, StrategyId, Variant,
	};

	fn create_test_flag() -> Flag {
		Flag {
			id: FlagId::new(),
			org_id: Some(OrgId::new()),
			key: "test.feature".to_string(),
			name: "Test Feature".to_string(),
			description: None,
			tags: vec![],
			maintainer_user_id: None,
			variants: vec![
				Variant {
					name: "off".to_string(),
					value: VariantValue::Boolean(false),
					weight: 50,
				},
				Variant {
					name: "on".to_string(),
					value: VariantValue::Boolean(true),
					weight: 50,
				},
			],
			default_variant: "off".to_string(),
			prerequisites: vec![],
			created_at: Utc::now(),
			updated_at: Utc::now(),
			archived_at: None,
		}
	}

	fn create_test_config(enabled: bool) -> FlagConfig {
		FlagConfig {
			id: FlagConfigId::new(),
			flag_id: FlagId::new(),
			environment_id: EnvironmentId::new(),
			enabled,
			strategy_id: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		}
	}

	#[test]
	fn test_evaluate_disabled_flag() {
		let flag = create_test_flag();
		let config = create_test_config(false);
		let context = EvaluationContext::new("prod");

		let result = evaluate_flag(&flag, Some(&config), None, &[], &[], &context);

		assert_eq!(result.variant, "off");
		assert_eq!(result.reason, EvaluationReason::Disabled);
	}

	#[test]
	fn test_evaluate_enabled_flag_no_strategy() {
		let flag = create_test_flag();
		let config = create_test_config(true);
		let context = EvaluationContext::new("prod");

		let result = evaluate_flag(&flag, Some(&config), None, &[], &[], &context);

		assert_eq!(result.variant, "off");
		assert_eq!(result.reason, EvaluationReason::Default);
	}

	#[test]
	fn test_evaluate_with_strategy() {
		let flag = create_test_flag();
		let config = create_test_config(true);
		let strategy = Strategy {
			id: StrategyId::new(),
			org_id: Some(OrgId::new()),
			name: "Everyone".to_string(),
			description: None,
			conditions: vec![],
			percentage: Some(100),
			percentage_key: PercentageKey::UserId,
			schedule: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};
		let context = EvaluationContext::new("prod").with_user_id("user123");

		let result = evaluate_flag(&flag, Some(&config), Some(&strategy), &[], &[], &context);

		// With 100% rollout and no conditions, should get non-default variant
		assert!(matches!(
			result.reason,
			EvaluationReason::Strategy { .. }
		));
	}

	#[test]
	fn test_evaluate_with_attribute_condition() {
		let flag = create_test_flag();
		let config = create_test_config(true);
		let strategy = Strategy {
			id: StrategyId::new(),
			org_id: Some(OrgId::new()),
			name: "Enterprise Only".to_string(),
			description: None,
			conditions: vec![Condition::Attribute {
				attribute: "plan".to_string(),
				operator: AttributeOperator::Equals,
				value: serde_json::json!("enterprise"),
			}],
			percentage: Some(100),
			percentage_key: PercentageKey::UserId,
			schedule: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		// User without enterprise plan
		let context = EvaluationContext::new("prod")
			.with_user_id("user123")
			.with_attribute("plan", serde_json::json!("free"));

		let result = evaluate_flag(&flag, Some(&config), Some(&strategy), &[], &[], &context);
		assert_eq!(result.reason, EvaluationReason::Default);

		// User with enterprise plan
		let context = EvaluationContext::new("prod")
			.with_user_id("user123")
			.with_attribute("plan", serde_json::json!("enterprise"));

		let result = evaluate_flag(&flag, Some(&config), Some(&strategy), &[], &[], &context);
		assert!(matches!(
			result.reason,
			EvaluationReason::Strategy { .. }
		));
	}

	#[test]
	fn test_evaluate_with_geo_condition() {
		let flag = create_test_flag();
		let config = create_test_config(true);
		let strategy = Strategy {
			id: StrategyId::new(),
			org_id: Some(OrgId::new()),
			name: "US Only".to_string(),
			description: None,
			conditions: vec![Condition::Geographic {
				field: GeoField::Country,
				operator: GeoOperator::In,
				values: vec!["US".to_string(), "CA".to_string()],
			}],
			percentage: Some(100),
			percentage_key: PercentageKey::UserId,
			schedule: None,
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		// User in Germany
		let context = EvaluationContext::new("prod")
			.with_user_id("user123")
			.with_geo(GeoContext::new().with_country("DE"));

		let result = evaluate_flag(&flag, Some(&config), Some(&strategy), &[], &[], &context);
		assert_eq!(result.reason, EvaluationReason::Default);

		// User in US
		let context = EvaluationContext::new("prod")
			.with_user_id("user123")
			.with_geo(GeoContext::new().with_country("US"));

		let result = evaluate_flag(&flag, Some(&config), Some(&strategy), &[], &[], &context);
		assert!(matches!(
			result.reason,
			EvaluationReason::Strategy { .. }
		));
	}

	#[test]
	fn test_evaluate_with_kill_switch() {
		use loom_flags_core::{KillSwitchId, UserId};

		let flag = create_test_flag();
		let config = create_test_config(true);

		let kill_switch = KillSwitch {
			id: KillSwitchId::new(),
			org_id: Some(OrgId::new()),
			key: "emergency_stop".to_string(),
			name: "Emergency Stop".to_string(),
			description: None,
			linked_flag_keys: vec!["test.feature".to_string()],
			is_active: true,
			activated_at: Some(Utc::now()),
			activated_by: Some(UserId::new()),
			activation_reason: Some("Testing".to_string()),
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		let context = EvaluationContext::new("prod");
		let result =
			evaluate_flag(&flag, Some(&config), None, &[kill_switch], &[], &context);

		assert_eq!(result.variant, "off");
		assert!(matches!(
			result.reason,
			EvaluationReason::KillSwitch { .. }
		));
	}

	#[test]
	fn test_percentage_consistent_hashing() {
		// The same key should always produce the same result
		let result1 = evaluate_percentage("user123", "test.feature", 50);
		let result2 = evaluate_percentage("user123", "test.feature", 50);
		assert_eq!(result1, result2);

		// Different keys may produce different results
		let results: Vec<bool> = (0..100)
			.map(|i| evaluate_percentage(&format!("user{}", i), "test.feature", 50))
			.collect();

		let true_count = results.iter().filter(|&&r| r).count();
		// Should be roughly 50% (with some tolerance)
		assert!(true_count > 30 && true_count < 70);
	}
}
