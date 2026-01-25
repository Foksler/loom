// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Feature flags HTTP handlers.
//!
//! Implements environment, SDK key, and flag management endpoints.

pub mod common;
pub mod configs;
pub mod environments;
pub mod evaluate;
pub mod flags_crud;
pub mod kill_switches;
pub mod sdk_keys;
pub mod stats;
pub mod strategies;
pub mod stream;

use loom_server_api::flags::FlagsErrorResponse;

use crate::impl_api_error_response;

impl_api_error_response!(FlagsErrorResponse);

// Re-export conversion helpers used by other routes (admin_flags)
pub use common::{
	condition_from_api, condition_to_api, percentage_key_from_api, percentage_key_to_api,
	schedule_from_api, schedule_to_api, sdk_key_type_from_api, sdk_key_type_to_api,
	variant_from_api, variant_to_api,
};

// Re-export all handlers
pub use configs::{get_flag_config, list_flag_configs, update_flag_config};
pub use environments::{
	create_environment, delete_environment, get_environment, list_environments, update_environment,
};
pub use evaluate::{evaluate_all_flags, evaluate_flag_endpoint};
pub use flags_crud::{archive_flag, create_flag, get_flag, list_flags, restore_flag, update_flag};
pub use kill_switches::{
	activate_kill_switch, create_kill_switch, deactivate_kill_switch, delete_kill_switch,
	get_kill_switch, list_kill_switches, update_kill_switch,
};
pub use sdk_keys::{create_sdk_key, list_sdk_keys, revoke_sdk_key};
pub use stats::{get_flag_stats, list_stale_flags};
pub use strategies::{
	create_strategy, delete_strategy, get_strategy, list_strategies, update_strategy,
};
pub use stream::{stream_flags, stream_stats, StreamFlagsParams};

// Re-export API types (these were publicly exported by the original flags.rs)
pub use loom_server_api::flags::{
	ActivateKillSwitchRequest, AttributeOperatorApi, ConditionApi, CreateEnvironmentRequest,
	CreateFlagRequest, CreateKillSwitchRequest, CreateSdkKeyRequest, CreateSdkKeyResponse,
	CreateStrategyRequest, EnvironmentResponse, EvaluateAllFlagsRequest, EvaluateAllFlagsResponse,
	EvaluateFlagRequest, EvaluationContextApi, EvaluationReasonApi, EvaluationResultApi,
	FlagConfigResponse, FlagPrerequisiteApi, FlagResponse, FlagStatsResponse, FlagsSuccessResponse,
	GeoContextApi, GeoFieldApi, GeoOperatorApi, KillSwitchResponse, ListEnvironmentsResponse,
	ListFlagConfigsResponse, ListFlagsQuery, ListFlagsResponse, ListKillSwitchesResponse,
	ListSdkKeysResponse, ListStaleFlagsResponse, ListStrategiesResponse, PercentageKeyApi,
	ScheduleApi, ScheduleStepApi, SdkKeyResponse, SdkKeyTypeApi, StaleFlagResponse,
	StrategyResponse, UpdateEnvironmentRequest, UpdateFlagConfigRequest, UpdateFlagRequest,
	UpdateKillSwitchRequest, UpdateStrategyRequest, VariantApi, VariantValueApi,
};
