// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! WhatsApp API types.

use serde::{Deserialize, Serialize};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Error response for WhatsApp endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WhatsAppErrorResponse {
	pub error: String,
	pub message: String,
}

/// Success response for WhatsApp endpoints.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WhatsAppSuccessResponse {
	pub message: String,
}

/// Request to create or update WhatsApp configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateWhatsAppConfigRequest {
	/// WhatsApp Business phone number ID from Meta dashboard.
	pub phone_number_id: String,
	/// Access token for WhatsApp Cloud API.
	pub access_token: String,
	/// App secret for webhook signature verification.
	pub app_secret: String,
	/// Verify token for webhook challenge verification.
	pub verify_token: String,
}

/// WhatsApp configuration response (secrets are masked).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WhatsAppConfigResponse {
	/// Configuration ID.
	pub id: String,
	/// Phone number ID (visible).
	pub phone_number_id: String,
	/// Whether the configuration is enabled.
	pub enabled: bool,
	/// Webhook URL for Meta dashboard.
	pub webhook_url: String,
	/// Created timestamp.
	pub created_at: String,
	/// Updated timestamp.
	pub updated_at: String,
}

/// Request to initiate phone linking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct LinkPhoneRequest {
	/// Phone number in E.164 format (e.g., +1234567890).
	pub phone_number: String,
}

/// Response after initiating phone linking.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct LinkPhoneResponse {
	/// Message indicating OTP was sent.
	pub message: String,
	/// Seconds until OTP expires.
	pub expires_in_seconds: i64,
}

/// Request to verify phone OTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct VerifyPhoneRequest {
	/// Phone number that was linked.
	pub phone_number: String,
	/// 6-digit OTP code.
	pub otp: String,
}

/// Response after successful phone verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct VerifyPhoneResponse {
	/// Success message.
	pub message: String,
	/// The verified phone number.
	pub phone_number: String,
}

/// WhatsApp group (conversation topic).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WhatsAppGroupResponse {
	pub id: String,
	pub name: String,
	pub description: Option<String>,
	pub color: Option<String>,
	pub is_default: bool,
	pub created_at: String,
	pub updated_at: String,
}

/// Request to create a WhatsApp group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct CreateWhatsAppGroupRequest {
	pub name: String,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub color: Option<String>,
	#[serde(default)]
	pub is_default: bool,
}

/// Request to update a WhatsApp group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct UpdateWhatsAppGroupRequest {
	#[serde(default)]
	pub name: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub color: Option<String>,
	#[serde(default)]
	pub is_default: Option<bool>,
}

/// List of WhatsApp groups.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListWhatsAppGroupsResponse {
	pub groups: Vec<WhatsAppGroupResponse>,
}

/// Request to move a conversation to a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct MoveConversationRequest {
	/// Group ID to move to, or null to unassign.
	pub group_id: Option<String>,
}

/// WhatsApp conversation summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct WhatsAppConversationResponse {
	pub id: String,
	pub wa_phone_number: String,
	pub group_id: Option<String>,
	pub user_id: Option<String>,
	pub thread_id: Option<String>,
	pub last_customer_message_at: String,
	pub session_expires_at: String,
	pub session_active: bool,
	pub status: String,
	pub created_at: String,
}

/// List of WhatsApp conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ListWhatsAppConversationsResponse {
	pub conversations: Vec<WhatsAppConversationResponse>,
}
