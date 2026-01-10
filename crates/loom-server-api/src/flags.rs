// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

// ============================================================================
// Environment Types
// ============================================================================

/// An environment in API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct EnvironmentResponse {
	/// Unique identifier for the environment.
	pub id: String,
	/// Organization ID this environment belongs to.
	pub org_id: String,
	/// Environment name (e.g., "dev", "prod").
	pub name: String,
	/// Optional hex color code (e.g., "#10b981").
	pub color: Option<String>,
	/// When the environment was created.
	pub created_at: DateTime<Utc>,
}

/// Request to create a new environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateEnvironmentRequest {
	/// Environment name (lowercase alphanumeric with underscores, 2-50 chars).
	pub name: String,
	/// Optional hex color code (e.g., "#10b981").
	pub color: Option<String>,
}

/// Request to update an environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateEnvironmentRequest {
	/// New environment name (optional).
	pub name: Option<String>,
	/// New hex color code (optional).
	pub color: Option<String>,
}

/// Response for listing environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListEnvironmentsResponse {
	pub environments: Vec<EnvironmentResponse>,
}

// ============================================================================
// SDK Key Types
// ============================================================================

/// SDK key type for API requests/responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum SdkKeyTypeApi {
	/// Safe for browser, single user context.
	ClientSide,
	/// Secret, backend only, any user context.
	ServerSide,
}

/// An SDK key in API responses (without the secret key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct SdkKeyResponse {
	/// Unique identifier for the SDK key.
	pub id: String,
	/// Environment ID this key belongs to.
	pub environment_id: String,
	/// Environment name for display.
	pub environment_name: String,
	/// Type of SDK key (client_side or server_side).
	pub key_type: SdkKeyTypeApi,
	/// Human-readable name for the key.
	pub name: String,
	/// User ID who created the key.
	pub created_by: String,
	/// When the key was created.
	pub created_at: DateTime<Utc>,
	/// When the key was last used.
	pub last_used_at: Option<DateTime<Utc>>,
	/// When the key was revoked (None if active).
	pub revoked_at: Option<DateTime<Utc>>,
}

/// Response for creating a new SDK key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateSdkKeyResponse {
	/// Unique identifier for the SDK key.
	pub id: String,
	/// The actual SDK key value (only shown once!).
	pub key: String,
	/// Environment ID this key belongs to.
	pub environment_id: String,
	/// Type of SDK key.
	pub key_type: SdkKeyTypeApi,
	/// Human-readable name for the key.
	pub name: String,
	/// When the key was created.
	pub created_at: DateTime<Utc>,
}

/// Request to create an SDK key.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateSdkKeyRequest {
	/// Environment ID to create the key for.
	pub environment_id: String,
	/// Type of SDK key (client_side or server_side).
	pub key_type: SdkKeyTypeApi,
	/// Human-readable name for the key.
	pub name: String,
}

/// Response for listing SDK keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListSdkKeysResponse {
	pub sdk_keys: Vec<SdkKeyResponse>,
}

// ============================================================================
// Common Response Types
// ============================================================================

/// Success response for flags operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FlagsSuccessResponse {
	pub message: String,
}

/// Error response for flags operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct FlagsErrorResponse {
	pub error: String,
	pub message: String,
}
