// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! WhatsApp message processing service.

use std::sync::Arc;

use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use loom_whatsapp::{
    InboundMessage, MessageDirection, MessageType, WebhookPayload, WhatsAppError,
};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use tracing::{debug, info, instrument, warn};

use crate::conversation::ConversationTracker;
use crate::repository::{WhatsAppConversationRow, WhatsAppRepository};

/// Result type for service operations.
pub type Result<T> = std::result::Result<T, WhatsAppError>;

/// WhatsApp message processing service.
#[derive(Debug, Clone)]
pub struct WhatsAppService {
    repo: Arc<WhatsAppRepository>,
    tracker: ConversationTracker,
}

impl WhatsAppService {
    /// Create a new service.
    pub fn new(repo: Arc<WhatsAppRepository>) -> Self {
        Self {
            repo,
            tracker: ConversationTracker::new(),
        }
    }

    /// Process an incoming webhook payload.
    #[instrument(skip(self, payload))]
    pub async fn process_webhook(&self, payload: WebhookPayload) -> Result<()> {
        for entry in payload.entry {
            for change in entry.changes {
                let phone_number_id = &change.value.metadata.phone_number_id;

                // Find config for this phone number
                let config = self
                    .repo
                    .find_config_by_phone_number_id(phone_number_id)
                    .await?
                    .ok_or_else(|| {
                        WhatsAppError::Config(format!(
                            "No config found for phone_number_id: {}",
                            phone_number_id
                        ))
                    })?;

                // Process messages
                for message in change.value.messages {
                    self.process_inbound_message(&config.id, message).await?;
                }

                // Process status updates
                for status in change.value.statuses {
                    debug!(
                        message_id = %status.id,
                        status = %serde_json::to_string(&status.status).unwrap_or_default(),
                        "Message status update"
                    );
                }
            }
        }

        Ok(())
    }

    /// Process a single inbound message.
    #[instrument(skip(self, message), fields(from = %message.from, wa_message_id = %message.id))]
    async fn process_inbound_message(
        &self,
        config_id: &str,
        message: InboundMessage,
    ) -> Result<()> {
        info!(message_type = ?message.message_type, "Processing inbound message");

        // Find or create conversation
        let conversation = self
            .repo
            .find_or_create_conversation(config_id, &message.from)
            .await?;

        // Extract message content
        let content = match &message.message_type {
            MessageType::Text => message.text.as_ref().map(|t| t.body.as_str()),
            MessageType::Image => message.image.as_ref().and_then(|m| m.caption.as_deref()),
            MessageType::Document => message.document.as_ref().and_then(|m| m.caption.as_deref()),
            _ => None,
        };

        // Parse timestamp
        let timestamp = message
            .timestamp
            .parse::<i64>()
            .map(|ts| chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(Utc::now))
            .unwrap_or_else(|_| Utc::now());

        // Store message
        self.repo
            .insert_message(
                &conversation.id,
                &message.id,
                MessageDirection::Inbound,
                message.message_type,
                content,
                timestamp,
            )
            .await?;

        info!(
            conversation_id = %conversation.id,
            "Message stored successfully"
        );

        Ok(())
    }

    /// Check if a session is active for a conversation.
    pub fn is_session_active(&self, conversation: &WhatsAppConversationRow) -> bool {
        self.tracker.is_session_active(conversation)
    }

    // =========================================================================
    // OTP Management
    // =========================================================================

    /// Generate a 6-digit OTP.
    pub fn generate_otp() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(0..1_000_000))
    }

    /// Hash a phone number for storage.
    pub fn hash_phone_number(phone: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(phone.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Hash an OTP using Argon2id.
    pub fn hash_otp(otp: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2
            .hash_password(otp.as_bytes(), &salt)
            .map_err(|e| WhatsAppError::VerificationFailed(e.to_string()))?;
        Ok(hash.to_string())
    }

    /// Verify an OTP against its hash.
    pub fn verify_otp(otp: &str, hash: &str) -> bool {
        let parsed_hash = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return false,
        };
        Argon2::default()
            .verify_password(otp.as_bytes(), &parsed_hash)
            .is_ok()
    }

    /// Create an OTP for phone verification.
    #[instrument(skip(self))]
    pub async fn create_phone_verification_otp(&self, user_id: &str, phone: &str) -> Result<String> {
        let phone_hash = Self::hash_phone_number(phone);

        // Check for recent OTP (cooldown)
        if let Some(recent) = self.repo.find_latest_otp_by_phone_hash(&phone_hash).await? {
            let cooldown = Duration::minutes(1);
            if recent.created_at + cooldown > Utc::now() {
                return Err(WhatsAppError::VerificationFailed(
                    "Please wait before requesting another code".into(),
                ));
            }
        }

        // Generate and store OTP
        let otp = Self::generate_otp();
        let otp_hash = Self::hash_otp(&otp)?;
        let expires_at = Utc::now() + Duration::minutes(5);

        self.repo
            .create_otp(user_id, &phone_hash, &otp_hash, expires_at)
            .await?;

        info!(phone_hash = %phone_hash, "OTP created for phone verification");
        Ok(otp)
    }

    /// Verify an OTP for phone linking.
    #[instrument(skip(self, otp))]
    pub async fn verify_phone_otp(&self, phone: &str, otp: &str) -> Result<String> {
        let phone_hash = Self::hash_phone_number(phone);

        let otp_record = self
            .repo
            .find_latest_otp_by_phone_hash(&phone_hash)
            .await?
            .ok_or(WhatsAppError::OtpInvalid)?;

        // Check if already used
        if otp_record.used_at.is_some() {
            return Err(WhatsAppError::OtpInvalid);
        }

        // Check if expired
        if otp_record.expires_at < Utc::now() {
            return Err(WhatsAppError::OtpExpired);
        }

        // Check attempts
        if otp_record.attempts >= 3 {
            return Err(WhatsAppError::OtpTooManyAttempts);
        }

        // Increment attempts
        self.repo.increment_otp_attempts(&otp_record.id).await?;

        // Verify OTP
        if !Self::verify_otp(otp, &otp_record.otp_hash) {
            warn!(attempts = otp_record.attempts + 1, "OTP verification failed");
            return Err(WhatsAppError::OtpInvalid);
        }

        // Mark as used
        self.repo.mark_otp_used(&otp_record.id).await?;

        info!("OTP verified successfully");
        Ok(otp_record.user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_otp() {
        let otp = WhatsAppService::generate_otp();
        assert_eq!(otp.len(), 6);
        assert!(otp.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_hash_phone_number() {
        let phone = "+1234567890";
        let hash1 = WhatsAppService::hash_phone_number(phone);
        let hash2 = WhatsAppService::hash_phone_number(phone);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn test_hash_and_verify_otp() {
        let otp = "123456";
        let hash = WhatsAppService::hash_otp(otp).unwrap();
        assert!(WhatsAppService::verify_otp(otp, &hash));
        assert!(!WhatsAppService::verify_otp("654321", &hash));
    }
}
