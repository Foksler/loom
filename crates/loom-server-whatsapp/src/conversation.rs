// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! Conversation tracking with 24-hour window enforcement.

use chrono::{DateTime, Duration, Utc};
use loom_whatsapp::WhatsAppError;

use crate::repository::WhatsAppConversationRow;

/// Tracks conversation state and 24-hour window.
#[derive(Debug, Clone)]
pub struct ConversationTracker {
    session_duration: Duration,
}

impl Default for ConversationTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationTracker {
    /// Create a new conversation tracker with default 24-hour window.
    pub fn new() -> Self {
        Self {
            session_duration: Duration::hours(24),
        }
    }

    /// Create a conversation tracker with custom session duration.
    pub fn with_session_duration(duration: Duration) -> Self {
        Self {
            session_duration: duration,
        }
    }

    /// Check if the session is still active.
    pub fn is_session_active(&self, conversation: &WhatsAppConversationRow) -> bool {
        conversation.session_expires_at > Utc::now()
    }

    /// Calculate the new session expiry time.
    pub fn calculate_session_expiry(&self) -> DateTime<Utc> {
        Utc::now() + self.session_duration
    }

    /// Get time remaining in session.
    pub fn time_remaining(&self, conversation: &WhatsAppConversationRow) -> Option<Duration> {
        let now = Utc::now();
        if conversation.session_expires_at > now {
            Some(conversation.session_expires_at - now)
        } else {
            None
        }
    }

    /// Validate that we can send a message in this conversation.
    pub fn validate_can_send(
        &self,
        conversation: &WhatsAppConversationRow,
    ) -> Result<(), WhatsAppError> {
        if !self.is_session_active(conversation) {
            return Err(WhatsAppError::SessionExpired);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_whatsapp::ConversationStatus;

    fn make_conversation(session_expires_at: DateTime<Utc>) -> WhatsAppConversationRow {
        WhatsAppConversationRow {
            id: "conv_123".to_string(),
            config_id: "cfg_123".to_string(),
            group_id: None,
            wa_phone_number: "+1234567890".to_string(),
            user_id: None,
            thread_id: None,
            last_customer_message_at: Utc::now(),
            session_expires_at,
            status: ConversationStatus::Active,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_session_active() {
        let tracker = ConversationTracker::new();

        // Active session
        let conv = make_conversation(Utc::now() + Duration::hours(12));
        assert!(tracker.is_session_active(&conv));

        // Expired session
        let conv = make_conversation(Utc::now() - Duration::hours(1));
        assert!(!tracker.is_session_active(&conv));
    }

    #[test]
    fn test_time_remaining() {
        let tracker = ConversationTracker::new();

        // Active session
        let expires = Utc::now() + Duration::hours(12);
        let conv = make_conversation(expires);
        let remaining = tracker.time_remaining(&conv);
        assert!(remaining.is_some());
        assert!(remaining.unwrap().num_hours() >= 11);

        // Expired session
        let conv = make_conversation(Utc::now() - Duration::hours(1));
        assert!(tracker.time_remaining(&conv).is_none());
    }

    #[test]
    fn test_validate_can_send() {
        let tracker = ConversationTracker::new();

        // Active session - can send
        let conv = make_conversation(Utc::now() + Duration::hours(12));
        assert!(tracker.validate_can_send(&conv).is_ok());

        // Expired session - cannot send
        let conv = make_conversation(Utc::now() - Duration::hours(1));
        assert!(matches!(
            tracker.validate_can_send(&conv),
            Err(WhatsAppError::SessionExpired)
        ));
    }
}
