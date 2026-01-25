// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Shared types and helpers for admin flags routes.

use loom_flags_core::{
	Flag, FlagPrerequisite, KillSwitch, Variant, VariantValue,
};
use serde::Deserialize;

// Re-export API types
pub use loom_server_api::flags::{
	ActivateKillSwitchRequest, CreateFlagRequest, CreateKillSwitchRequest, CreateStrategyRequest,
	FlagPrerequisiteApi, FlagResponse, FlagsErrorResponse, FlagsSuccessResponse, KillSwitchResponse,
	ListFlagsResponse, ListKillSwitchesResponse, ListStrategiesResponse, StrategyResponse,
	UpdateFlagRequest, UpdateKillSwitchRequest, UpdateStrategyRequest, VariantApi, VariantValueApi,
};

/// Query parameters for listing platform flags.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PlatformFlagsQuery {
	/// Include archived flags (default: false).
	#[serde(default)]
	pub include_archived: bool,
}

pub fn variant_from_api(v: &VariantApi) -> Variant {
	Variant {
		name: v.name.clone(),
		value: match &v.value {
			VariantValueApi::Boolean(b) => VariantValue::Boolean(*b),
			VariantValueApi::String(s) => VariantValue::String(s.clone()),
			VariantValueApi::Json(j) => VariantValue::Json(j.clone()),
		},
		weight: v.weight,
	}
}

pub fn variant_to_api(v: &Variant) -> VariantApi {
	VariantApi {
		name: v.name.clone(),
		value: match &v.value {
			VariantValue::Boolean(b) => VariantValueApi::Boolean(*b),
			VariantValue::String(s) => VariantValueApi::String(s.clone()),
			VariantValue::Json(j) => VariantValueApi::Json(j.clone()),
		},
		weight: v.weight,
	}
}

pub fn prerequisite_from_api(p: &FlagPrerequisiteApi) -> FlagPrerequisite {
	FlagPrerequisite {
		flag_key: p.flag_key.clone(),
		required_variant: p.required_variant.clone(),
	}
}

pub fn prerequisite_to_api(p: &FlagPrerequisite) -> FlagPrerequisiteApi {
	FlagPrerequisiteApi {
		flag_key: p.flag_key.clone(),
		required_variant: p.required_variant.clone(),
	}
}

pub fn flag_to_response(flag: &Flag) -> FlagResponse {
	FlagResponse {
		id: flag.id.to_string(),
		org_id: flag.org_id.map(|id| id.to_string()),
		key: flag.key.clone(),
		name: flag.name.clone(),
		description: flag.description.clone(),
		tags: flag.tags.clone(),
		maintainer_user_id: flag.maintainer_user_id.map(|id| id.to_string()),
		variants: flag.variants.iter().map(variant_to_api).collect(),
		default_variant: flag.default_variant.clone(),
		prerequisites: flag.prerequisites.iter().map(prerequisite_to_api).collect(),
		is_archived: flag.is_archived(),
		created_at: flag.created_at,
		updated_at: flag.updated_at,
		archived_at: flag.archived_at,
	}
}

pub fn kill_switch_to_response(ks: &KillSwitch) -> KillSwitchResponse {
	KillSwitchResponse {
		id: ks.id.to_string(),
		org_id: ks.org_id.map(|id| id.to_string()),
		key: ks.key.clone(),
		name: ks.name.clone(),
		description: ks.description.clone(),
		linked_flag_keys: ks.linked_flag_keys.clone(),
		is_active: ks.is_active,
		activated_at: ks.activated_at,
		activated_by: ks.activated_by.map(|id| id.to_string()),
		activation_reason: ks.activation_reason.clone(),
		created_at: ks.created_at,
		updated_at: ks.updated_at,
	}
}
