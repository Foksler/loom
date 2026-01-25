// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for flags routes.

use axum::response::IntoResponse;
use loom_flags_core::{
	AttributeOperator, Condition, GeoField, GeoOperator, PercentageKey, Schedule, ScheduleStep,
	SdkKeyType, Variant, VariantValue, FlagPrerequisite,
};
use loom_server_api::flags::{
	AttributeOperatorApi, ConditionApi, FlagPrerequisiteApi, FlagsErrorResponse, GeoFieldApi,
	GeoOperatorApi, PercentageKeyApi, ScheduleApi, ScheduleStepApi, SdkKeyTypeApi, VariantApi,
	VariantValueApi,
};

use crate::{
	api::AppState,
	api_response::{internal_error, not_found},
	i18n::t,
};

// ============================================================================
// Org Membership Check
// ============================================================================

/// Check if user is a member of the organization.
/// Returns Ok(()) if member, Err response otherwise.
pub async fn check_org_membership(
	state: &AppState,
	org_id: &loom_server_auth::types::OrgId,
	user_id: &loom_server_auth::types::UserId,
	locale: &str,
) -> Result<(), axum::response::Response> {
	match state.org_repo.get_membership(org_id, user_id).await {
		Ok(Some(_)) => Ok(()),
		Ok(None) => Err(not_found::<FlagsErrorResponse>(t(locale, "server.api.org.not_a_member"))
			.into_response()),
		Err(e) => {
			tracing::error!(error = %e, %org_id, "Failed to check org membership");
			Err(internal_error::<FlagsErrorResponse>(t(locale, "server.api.error.internal"))
				.into_response())
		}
	}
}

// ============================================================================
// SDK Key Type Conversions
// ============================================================================

pub fn sdk_key_type_to_api(kt: SdkKeyType) -> SdkKeyTypeApi {
	match kt {
		SdkKeyType::ClientSide => SdkKeyTypeApi::ClientSide,
		SdkKeyType::ServerSide => SdkKeyTypeApi::ServerSide,
	}
}

pub fn sdk_key_type_from_api(kt: SdkKeyTypeApi) -> SdkKeyType {
	match kt {
		SdkKeyTypeApi::ClientSide => SdkKeyType::ClientSide,
		SdkKeyTypeApi::ServerSide => SdkKeyType::ServerSide,
	}
}

// ============================================================================
// Variant Conversions
// ============================================================================

pub fn variant_to_api(v: &Variant) -> VariantApi {
	VariantApi {
		name: v.name.clone(),
		value: variant_value_to_api(&v.value),
		weight: v.weight,
	}
}

pub fn variant_value_to_api(v: &VariantValue) -> VariantValueApi {
	match v {
		VariantValue::Boolean(b) => VariantValueApi::Boolean(*b),
		VariantValue::String(s) => VariantValueApi::String(s.clone()),
		VariantValue::Json(j) => VariantValueApi::Json(j.clone()),
	}
}

pub fn variant_from_api(v: &VariantApi) -> Variant {
	Variant {
		name: v.name.clone(),
		value: variant_value_from_api(&v.value),
		weight: v.weight,
	}
}

pub fn variant_value_from_api(v: &VariantValueApi) -> VariantValue {
	match v {
		VariantValueApi::Boolean(b) => VariantValue::Boolean(*b),
		VariantValueApi::String(s) => VariantValue::String(s.clone()),
		VariantValueApi::Json(j) => VariantValue::Json(j.clone()),
	}
}

// ============================================================================
// Prerequisite Conversions
// ============================================================================

pub fn prerequisite_to_api(p: &FlagPrerequisite) -> FlagPrerequisiteApi {
	FlagPrerequisiteApi {
		flag_key: p.flag_key.clone(),
		required_variant: p.required_variant.clone(),
	}
}

pub fn prerequisite_from_api(p: &FlagPrerequisiteApi) -> FlagPrerequisite {
	FlagPrerequisite {
		flag_key: p.flag_key.clone(),
		required_variant: p.required_variant.clone(),
	}
}

// ============================================================================
// Condition Conversions
// ============================================================================

pub fn condition_to_api(c: &Condition) -> ConditionApi {
	match c {
		Condition::Attribute {
			attribute,
			operator,
			value,
		} => ConditionApi::Attribute {
			attribute: attribute.clone(),
			operator: attribute_operator_to_api(*operator),
			value: value.clone(),
		},
		Condition::Geographic {
			field,
			operator,
			values,
		} => ConditionApi::Geographic {
			field: geo_field_to_api(*field),
			operator: geo_operator_to_api(*operator),
			values: values.clone(),
		},
		Condition::Environment { environments } => ConditionApi::Environment {
			environments: environments.clone(),
		},
	}
}

pub fn condition_from_api(c: &ConditionApi) -> Condition {
	match c {
		ConditionApi::Attribute {
			attribute,
			operator,
			value,
		} => Condition::Attribute {
			attribute: attribute.clone(),
			operator: attribute_operator_from_api(*operator),
			value: value.clone(),
		},
		ConditionApi::Geographic {
			field,
			operator,
			values,
		} => Condition::Geographic {
			field: geo_field_from_api(*field),
			operator: geo_operator_from_api(*operator),
			values: values.clone(),
		},
		ConditionApi::Environment { environments } => Condition::Environment {
			environments: environments.clone(),
		},
	}
}

// ============================================================================
// Attribute Operator Conversions
// ============================================================================

pub fn attribute_operator_to_api(op: AttributeOperator) -> AttributeOperatorApi {
	match op {
		AttributeOperator::Equals => AttributeOperatorApi::Equals,
		AttributeOperator::NotEquals => AttributeOperatorApi::NotEquals,
		AttributeOperator::Contains => AttributeOperatorApi::Contains,
		AttributeOperator::StartsWith => AttributeOperatorApi::StartsWith,
		AttributeOperator::EndsWith => AttributeOperatorApi::EndsWith,
		AttributeOperator::GreaterThan => AttributeOperatorApi::GreaterThan,
		AttributeOperator::LessThan => AttributeOperatorApi::LessThan,
		AttributeOperator::GreaterThanOrEquals => AttributeOperatorApi::GreaterThanOrEquals,
		AttributeOperator::LessThanOrEquals => AttributeOperatorApi::LessThanOrEquals,
		AttributeOperator::In => AttributeOperatorApi::In,
		AttributeOperator::NotIn => AttributeOperatorApi::NotIn,
	}
}

pub fn attribute_operator_from_api(op: AttributeOperatorApi) -> AttributeOperator {
	match op {
		AttributeOperatorApi::Equals => AttributeOperator::Equals,
		AttributeOperatorApi::NotEquals => AttributeOperator::NotEquals,
		AttributeOperatorApi::Contains => AttributeOperator::Contains,
		AttributeOperatorApi::StartsWith => AttributeOperator::StartsWith,
		AttributeOperatorApi::EndsWith => AttributeOperator::EndsWith,
		AttributeOperatorApi::GreaterThan => AttributeOperator::GreaterThan,
		AttributeOperatorApi::LessThan => AttributeOperator::LessThan,
		AttributeOperatorApi::GreaterThanOrEquals => AttributeOperator::GreaterThanOrEquals,
		AttributeOperatorApi::LessThanOrEquals => AttributeOperator::LessThanOrEquals,
		AttributeOperatorApi::In => AttributeOperator::In,
		AttributeOperatorApi::NotIn => AttributeOperator::NotIn,
	}
}

// ============================================================================
// Geo Field/Operator Conversions
// ============================================================================

pub fn geo_field_to_api(f: GeoField) -> GeoFieldApi {
	match f {
		GeoField::Country => GeoFieldApi::Country,
		GeoField::Region => GeoFieldApi::Region,
		GeoField::City => GeoFieldApi::City,
	}
}

pub fn geo_field_from_api(f: GeoFieldApi) -> GeoField {
	match f {
		GeoFieldApi::Country => GeoField::Country,
		GeoFieldApi::Region => GeoField::Region,
		GeoFieldApi::City => GeoField::City,
	}
}

pub fn geo_operator_to_api(op: GeoOperator) -> GeoOperatorApi {
	match op {
		GeoOperator::In => GeoOperatorApi::In,
		GeoOperator::NotIn => GeoOperatorApi::NotIn,
	}
}

pub fn geo_operator_from_api(op: GeoOperatorApi) -> GeoOperator {
	match op {
		GeoOperatorApi::In => GeoOperator::In,
		GeoOperatorApi::NotIn => GeoOperator::NotIn,
	}
}

// ============================================================================
// Percentage Key Conversions
// ============================================================================

pub fn percentage_key_to_api(pk: &PercentageKey) -> PercentageKeyApi {
	match pk {
		PercentageKey::UserId => PercentageKeyApi::UserId,
		PercentageKey::OrgId => PercentageKeyApi::OrgId,
		PercentageKey::SessionId => PercentageKeyApi::SessionId,
		PercentageKey::Custom(s) => PercentageKeyApi::Custom(s.clone()),
	}
}

pub fn percentage_key_from_api(pk: &PercentageKeyApi) -> PercentageKey {
	match pk {
		PercentageKeyApi::UserId => PercentageKey::UserId,
		PercentageKeyApi::OrgId => PercentageKey::OrgId,
		PercentageKeyApi::SessionId => PercentageKey::SessionId,
		PercentageKeyApi::Custom(s) => PercentageKey::Custom(s.clone()),
	}
}

// ============================================================================
// Schedule Conversions
// ============================================================================

pub fn schedule_to_api(s: &Schedule) -> ScheduleApi {
	ScheduleApi {
		steps: s.steps.iter().map(schedule_step_to_api).collect(),
	}
}

pub fn schedule_from_api(s: &ScheduleApi) -> Schedule {
	Schedule {
		steps: s.steps.iter().map(schedule_step_from_api).collect(),
	}
}

pub fn schedule_step_to_api(s: &ScheduleStep) -> ScheduleStepApi {
	ScheduleStepApi {
		percentage: s.percentage,
		start_at: s.start_at,
	}
}

pub fn schedule_step_from_api(s: &ScheduleStepApi) -> ScheduleStep {
	ScheduleStep {
		percentage: s.percentage,
		start_at: s.start_at,
	}
}
