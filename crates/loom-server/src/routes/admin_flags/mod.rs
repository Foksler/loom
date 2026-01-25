// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! Admin routes for platform-level feature flags.
//!
//! Provides endpoints for super admin management of platform-wide flags:
//! - List, create, update, archive platform flags
//! - List, create, update, delete platform kill switches
//! - Activate/deactivate platform kill switches
//!
//! Platform flags have `org_id = None` and override org-level flags with the
//! same key during evaluation.
//!
//! # Security
//!
//! All endpoints require `system_admin` role.

pub mod common;
pub mod flags;
pub mod kill_switches;
pub mod strategies;

// Re-export all handlers and their utoipa path types
pub use flags::{
	__path_archive_platform_flag, __path_create_platform_flag, __path_get_platform_flag,
	__path_list_platform_flags, __path_restore_platform_flag, __path_update_platform_flag,
	archive_platform_flag, create_platform_flag, get_platform_flag, list_platform_flags,
	restore_platform_flag, update_platform_flag,
};
pub use kill_switches::{
	__path_activate_platform_kill_switch, __path_create_platform_kill_switch,
	__path_deactivate_platform_kill_switch, __path_delete_platform_kill_switch,
	__path_get_platform_kill_switch, __path_list_platform_kill_switches,
	__path_update_platform_kill_switch, activate_platform_kill_switch,
	create_platform_kill_switch, deactivate_platform_kill_switch, delete_platform_kill_switch,
	get_platform_kill_switch, list_platform_kill_switches, update_platform_kill_switch,
};
pub use strategies::{__path_list_platform_strategies, list_platform_strategies};

// Re-export API types for external use
pub use common::{
	ActivateKillSwitchRequest, CreateFlagRequest, CreateKillSwitchRequest, CreateStrategyRequest,
	FlagPrerequisiteApi, FlagResponse, FlagsErrorResponse, FlagsSuccessResponse, KillSwitchResponse,
	ListFlagsResponse, ListKillSwitchesResponse, ListStrategiesResponse, PlatformFlagsQuery,
	StrategyResponse, UpdateFlagRequest, UpdateKillSwitchRequest, UpdateStrategyRequest, VariantApi,
	VariantValueApi,
};
