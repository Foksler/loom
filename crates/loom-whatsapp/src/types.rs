// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! WhatsApp Cloud API types for webhooks and messaging.

use serde::{Deserialize, Serialize};

/// Webhook verification query parameters (GET request).
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookVerifyParams {
    #[serde(rename = "hub.mode")]
    pub mode: String,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub challenge: String,
}

/// Incoming webhook payload (POST request).
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookPayload {
    pub object: String,
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookChange {
    pub value: WebhookValue,
    pub field: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: String,
    pub metadata: WebhookMetadata,
    #[serde(default)]
    pub messages: Vec<InboundMessage>,
    #[serde(default)]
    pub statuses: Vec<MessageStatus>,
    #[serde(default)]
    pub contacts: Vec<WebhookContact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookMetadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebhookContact {
    pub profile: ContactProfile,
    pub wa_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactProfile {
    pub name: String,
}

/// Inbound message from a customer.
#[derive(Debug, Clone, Deserialize)]
pub struct InboundMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: MessageType,
    #[serde(default)]
    pub text: Option<TextMessage>,
    #[serde(default)]
    pub image: Option<MediaMessage>,
    #[serde(default)]
    pub document: Option<MediaMessage>,
    #[serde(default)]
    pub audio: Option<MediaMessage>,
    #[serde(default)]
    pub video: Option<MediaMessage>,
    #[serde(default)]
    pub sticker: Option<MediaMessage>,
    #[serde(default)]
    pub location: Option<LocationMessage>,
    #[serde(default)]
    pub contacts: Option<Vec<ContactMessage>>,
    #[serde(default)]
    pub context: Option<MessageContext>,
}

/// Message type enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Image,
    Document,
    Audio,
    Video,
    Sticker,
    Location,
    Contacts,
    Interactive,
    Button,
    Unknown,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TextMessage {
    pub body: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaMessage {
    pub id: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub caption: Option<String>,
    #[serde(default)]
    pub filename: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LocationMessage {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactMessage {
    pub name: ContactName,
    #[serde(default)]
    pub phones: Vec<ContactPhone>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactName {
    pub formatted_name: String,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactPhone {
    pub phone: String,
    #[serde(rename = "type", default)]
    pub phone_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageContext {
    pub from: String,
    pub id: String,
}

/// Message delivery status update.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageStatus {
    pub id: String,
    pub status: DeliveryStatus,
    pub timestamp: String,
    pub recipient_id: String,
    #[serde(default)]
    pub conversation: Option<ConversationInfo>,
    #[serde(default)]
    pub pricing: Option<PricingInfo>,
    #[serde(default)]
    pub errors: Vec<StatusError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryStatus {
    Sent,
    Delivered,
    Read,
    Failed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationInfo {
    pub id: String,
    #[serde(default)]
    pub origin: Option<ConversationOrigin>,
    #[serde(default)]
    pub expiration_timestamp: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationOrigin {
    #[serde(rename = "type")]
    pub origin_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PricingInfo {
    pub billable: bool,
    pub pricing_model: String,
    pub category: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusError {
    pub code: i32,
    pub title: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub error_data: Option<serde_json::Value>,
}

// ============================================================================
// Outbound message types
// ============================================================================

/// Request to send a message.
#[derive(Debug, Clone, Serialize)]
pub struct SendMessageRequest {
    pub messaging_product: String,
    pub recipient_type: String,
    pub to: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<OutboundText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<OutboundMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<OutboundMedia>,
}

impl SendMessageRequest {
    /// Create a text message request.
    pub fn text(to: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            messaging_product: "whatsapp".to_string(),
            recipient_type: "individual".to_string(),
            to: to.into(),
            message_type: "text".to_string(),
            text: Some(OutboundText {
                preview_url: false,
                body: body.into(),
            }),
            image: None,
            document: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutboundText {
    pub preview_url: bool,
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutboundMedia {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
}

/// Response from sending a message.
#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageResponse {
    pub messaging_product: String,
    pub contacts: Vec<ResponseContact>,
    pub messages: Vec<SentMessage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResponseContact {
    pub input: String,
    pub wa_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SentMessage {
    pub id: String,
    #[serde(default)]
    pub message_status: Option<String>,
}

// ============================================================================
// Database entity types
// ============================================================================

/// Direction of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

/// Conversation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConversationStatus {
    Active,
    Expired,
    Archived,
}

impl Default for ConversationStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Message delivery/processing status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageProcessingStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
}

impl Default for MessageProcessingStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_message_request_text() {
        let req = SendMessageRequest::text("+1234567890", "Hello, World!");
        assert_eq!(req.messaging_product, "whatsapp");
        assert_eq!(req.recipient_type, "individual");
        assert_eq!(req.to, "+1234567890");
        assert_eq!(req.message_type, "text");
        assert!(req.text.is_some());
        assert_eq!(req.text.unwrap().body, "Hello, World!");
    }

    #[test]
    fn test_parse_webhook_payload() {
        let json = r#"{
            "object": "whatsapp_business_account",
            "entry": [{
                "id": "123456789",
                "changes": [{
                    "value": {
                        "messaging_product": "whatsapp",
                        "metadata": {
                            "display_phone_number": "+1234567890",
                            "phone_number_id": "987654321"
                        },
                        "messages": [{
                            "from": "+0987654321",
                            "id": "wamid.abc123",
                            "timestamp": "1234567890",
                            "type": "text",
                            "text": { "body": "Hello!" }
                        }]
                    },
                    "field": "messages"
                }]
            }]
        }"#;

        let payload: WebhookPayload = serde_json::from_str(json).unwrap();
        assert_eq!(payload.object, "whatsapp_business_account");
        assert_eq!(payload.entry.len(), 1);
        let change = &payload.entry[0].changes[0];
        assert_eq!(change.value.messages.len(), 1);
        assert_eq!(change.value.messages[0].message_type, MessageType::Text);
    }
}
