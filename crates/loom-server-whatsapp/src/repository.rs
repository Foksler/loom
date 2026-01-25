// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
// SPDX-License-Identifier: Proprietary

//! SQLite repository for WhatsApp entities.

use chrono::{DateTime, Duration, Utc};
use loom_whatsapp::{
    ConversationStatus, MessageDirection, MessageProcessingStatus, MessageType, WhatsAppError,
};
use sqlx::{Row, SqlitePool};
use tracing::instrument;
use uuid::Uuid;

/// Parse a datetime string from SQLite.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Result type for repository operations.
pub type Result<T> = std::result::Result<T, WhatsAppError>;

/// WhatsApp configuration stored in database.
#[derive(Debug, Clone)]
pub struct WhatsAppConfigRow {
    pub id: String,
    pub org_id: String,
    pub phone_number_id: String,
    pub access_token_encrypted: String,
    pub app_secret_hash: String,
    pub verify_token_hash: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// WhatsApp conversation group.
#[derive(Debug, Clone)]
pub struct WhatsAppGroupRow {
    pub id: String,
    pub config_id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// WhatsApp conversation.
#[derive(Debug, Clone)]
pub struct WhatsAppConversationRow {
    pub id: String,
    pub config_id: String,
    pub group_id: Option<String>,
    pub wa_phone_number: String,
    pub user_id: Option<String>,
    pub thread_id: Option<String>,
    pub last_customer_message_at: DateTime<Utc>,
    pub session_expires_at: DateTime<Utc>,
    pub status: ConversationStatus,
    pub created_at: DateTime<Utc>,
}

/// WhatsApp message.
#[derive(Debug, Clone)]
pub struct WhatsAppMessageRow {
    pub id: String,
    pub conversation_id: String,
    pub wa_message_id: String,
    pub direction: MessageDirection,
    pub message_type: MessageType,
    pub content: Option<String>,
    pub media_url: Option<String>,
    pub status: MessageProcessingStatus,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// WhatsApp OTP for phone verification.
#[derive(Debug, Clone)]
pub struct WhatsAppOtpRow {
    pub id: String,
    pub user_id: String,
    pub phone_number_hash: String,
    pub otp_hash: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub attempts: i32,
}

/// SQLite repository for WhatsApp entities.
#[derive(Debug, Clone)]
pub struct WhatsAppRepository {
    pool: SqlitePool,
}

impl WhatsAppRepository {
    /// Create a new repository.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // Config operations
    // =========================================================================

    /// Find config by organization ID.
    #[instrument(skip(self))]
    pub async fn find_config_by_org_id(&self, org_id: &str) -> Result<Option<WhatsAppConfigRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, org_id, phone_number_id, access_token_encrypted,
                   app_secret_hash, verify_token_hash, enabled, created_at, updated_at
            FROM whatsapp_configs
            WHERE org_id = ?
            "#,
        )
        .bind(org_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.map(|r| WhatsAppConfigRow {
            id: r.get("id"),
            org_id: r.get("org_id"),
            phone_number_id: r.get("phone_number_id"),
            access_token_encrypted: r.get("access_token_encrypted"),
            app_secret_hash: r.get("app_secret_hash"),
            verify_token_hash: r.get("verify_token_hash"),
            enabled: r.get::<i32, _>("enabled") == 1,
            created_at: parse_datetime(&r.get::<String, _>("created_at")),
            updated_at: parse_datetime(&r.get::<String, _>("updated_at")),
        }))
    }

    /// Find config by phone number ID.
    #[instrument(skip(self))]
    pub async fn find_config_by_phone_number_id(
        &self,
        phone_number_id: &str,
    ) -> Result<Option<WhatsAppConfigRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, org_id, phone_number_id, access_token_encrypted,
                   app_secret_hash, verify_token_hash, enabled, created_at, updated_at
            FROM whatsapp_configs
            WHERE phone_number_id = ? AND enabled = 1
            "#,
        )
        .bind(phone_number_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.map(|r| WhatsAppConfigRow {
            id: r.get("id"),
            org_id: r.get("org_id"),
            phone_number_id: r.get("phone_number_id"),
            access_token_encrypted: r.get("access_token_encrypted"),
            app_secret_hash: r.get("app_secret_hash"),
            verify_token_hash: r.get("verify_token_hash"),
            enabled: r.get::<i32, _>("enabled") == 1,
            created_at: parse_datetime(&r.get::<String, _>("created_at")),
            updated_at: parse_datetime(&r.get::<String, _>("updated_at")),
        }))
    }

    /// Find config by verify token hash.
    #[instrument(skip(self))]
    pub async fn find_config_by_verify_token_hash(
        &self,
        hash: &str,
    ) -> Result<Option<WhatsAppConfigRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, org_id, phone_number_id, access_token_encrypted,
                   app_secret_hash, verify_token_hash, enabled, created_at, updated_at
            FROM whatsapp_configs
            WHERE verify_token_hash = ? AND enabled = 1
            "#,
        )
        .bind(hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.map(|r| WhatsAppConfigRow {
            id: r.get("id"),
            org_id: r.get("org_id"),
            phone_number_id: r.get("phone_number_id"),
            access_token_encrypted: r.get("access_token_encrypted"),
            app_secret_hash: r.get("app_secret_hash"),
            verify_token_hash: r.get("verify_token_hash"),
            enabled: r.get::<i32, _>("enabled") == 1,
            created_at: parse_datetime(&r.get::<String, _>("created_at")),
            updated_at: parse_datetime(&r.get::<String, _>("updated_at")),
        }))
    }

    /// List all enabled configs.
    #[instrument(skip(self))]
    pub async fn list_enabled_configs(&self) -> Result<Vec<WhatsAppConfigRow>> {
        let rows = sqlx::query(
            r#"
            SELECT id, org_id, phone_number_id, access_token_encrypted,
                   app_secret_hash, verify_token_hash, enabled, created_at, updated_at
            FROM whatsapp_configs
            WHERE enabled = 1
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| WhatsAppConfigRow {
                id: r.get("id"),
                org_id: r.get("org_id"),
                phone_number_id: r.get("phone_number_id"),
                access_token_encrypted: r.get("access_token_encrypted"),
                app_secret_hash: r.get("app_secret_hash"),
                verify_token_hash: r.get("verify_token_hash"),
                enabled: r.get::<i32, _>("enabled") == 1,
                created_at: parse_datetime(&r.get::<String, _>("created_at")),
                updated_at: parse_datetime(&r.get::<String, _>("updated_at")),
            })
            .collect())
    }

    /// Insert or update a config.
    #[instrument(skip(self, access_token_encrypted))]
    pub async fn upsert_config(
        &self,
        org_id: &str,
        phone_number_id: &str,
        access_token_encrypted: &str,
        app_secret_hash: &str,
        verify_token_hash: &str,
    ) -> Result<WhatsAppConfigRow> {
        let now = Utc::now();
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO whatsapp_configs (id, org_id, phone_number_id, access_token_encrypted,
                                          app_secret_hash, verify_token_hash, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 1, ?, ?)
            ON CONFLICT(org_id) DO UPDATE SET
                phone_number_id = excluded.phone_number_id,
                access_token_encrypted = excluded.access_token_encrypted,
                app_secret_hash = excluded.app_secret_hash,
                verify_token_hash = excluded.verify_token_hash,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&id)
        .bind(org_id)
        .bind(phone_number_id)
        .bind(access_token_encrypted)
        .bind(app_secret_hash)
        .bind(verify_token_hash)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        self.find_config_by_org_id(org_id)
            .await?
            .ok_or_else(|| WhatsAppError::Config("Failed to retrieve config after upsert".into()))
    }

    /// Delete a config.
    #[instrument(skip(self))]
    pub async fn delete_config(&self, org_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM whatsapp_configs WHERE org_id = ?")
            .bind(org_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;
        Ok(())
    }

    /// Count enabled configs (for health check).
    #[instrument(skip(self))]
    pub async fn count_enabled_configs(&self) -> Result<usize> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM whatsapp_configs WHERE enabled = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.get::<i32, _>("count") as usize)
    }

    // =========================================================================
    // Conversation operations
    // =========================================================================

    /// Find or create a conversation.
    #[instrument(skip(self))]
    pub async fn find_or_create_conversation(
        &self,
        config_id: &str,
        wa_phone_number: &str,
    ) -> Result<WhatsAppConversationRow> {
        let now = Utc::now();
        let session_expires = now + Duration::hours(24);
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT INTO whatsapp_conversations (id, config_id, wa_phone_number,
                                                last_customer_message_at, session_expires_at,
                                                status, created_at)
            VALUES (?, ?, ?, ?, ?, 'active', ?)
            ON CONFLICT(config_id, wa_phone_number) DO UPDATE SET
                last_customer_message_at = excluded.last_customer_message_at,
                session_expires_at = excluded.session_expires_at,
                status = 'active'
            "#,
        )
        .bind(&id)
        .bind(config_id)
        .bind(wa_phone_number)
        .bind(now.to_rfc3339())
        .bind(session_expires.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        self.find_conversation_by_phone(config_id, wa_phone_number)
            .await?
            .ok_or_else(|| {
                WhatsAppError::Config("Failed to retrieve conversation after upsert".into())
            })
    }

    /// Find conversation by phone number.
    #[instrument(skip(self))]
    pub async fn find_conversation_by_phone(
        &self,
        config_id: &str,
        wa_phone_number: &str,
    ) -> Result<Option<WhatsAppConversationRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, config_id, group_id, wa_phone_number, user_id, thread_id,
                   last_customer_message_at, session_expires_at, status, created_at
            FROM whatsapp_conversations
            WHERE config_id = ? AND wa_phone_number = ?
            "#,
        )
        .bind(config_id)
        .bind(wa_phone_number)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.map(|r| WhatsAppConversationRow {
            id: r.get("id"),
            config_id: r.get("config_id"),
            group_id: r.get("group_id"),
            wa_phone_number: r.get("wa_phone_number"),
            user_id: r.get("user_id"),
            thread_id: r.get("thread_id"),
            last_customer_message_at: parse_datetime(&r.get::<String, _>("last_customer_message_at")),
            session_expires_at: parse_datetime(&r.get::<String, _>("session_expires_at")),
            status: serde_json::from_str(&r.get::<String, _>("status"))
                .unwrap_or(ConversationStatus::Active),
            created_at: parse_datetime(&r.get::<String, _>("created_at")),
        }))
    }

    /// Link conversation to a thread.
    #[instrument(skip(self))]
    pub async fn link_conversation_to_thread(
        &self,
        conversation_id: &str,
        thread_id: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE whatsapp_conversations SET thread_id = ? WHERE id = ?")
            .bind(thread_id)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;
        Ok(())
    }

    /// Link conversation to a user.
    #[instrument(skip(self))]
    pub async fn link_conversation_to_user(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE whatsapp_conversations SET user_id = ? WHERE id = ?")
            .bind(user_id)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;
        Ok(())
    }

    // =========================================================================
    // Message operations
    // =========================================================================

    /// Insert a message.
    #[instrument(skip(self))]
    pub async fn insert_message(
        &self,
        conversation_id: &str,
        wa_message_id: &str,
        direction: MessageDirection,
        message_type: MessageType,
        content: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> Result<WhatsAppMessageRow> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO whatsapp_messages (id, conversation_id, wa_message_id, direction,
                                           message_type, content, status, timestamp, created_at)
            VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, ?)
            "#,
        )
        .bind(&id)
        .bind(conversation_id)
        .bind(wa_message_id)
        .bind(serde_json::to_string(&direction).unwrap())
        .bind(serde_json::to_string(&message_type).unwrap())
        .bind(content)
        .bind(timestamp.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(WhatsAppMessageRow {
            id,
            conversation_id: conversation_id.to_string(),
            wa_message_id: wa_message_id.to_string(),
            direction,
            message_type,
            content: content.map(String::from),
            media_url: None,
            status: MessageProcessingStatus::Pending,
            timestamp,
            created_at: now,
        })
    }

    // =========================================================================
    // OTP operations
    // =========================================================================

    /// Create an OTP record.
    #[instrument(skip(self, otp_hash))]
    pub async fn create_otp(
        &self,
        user_id: &str,
        phone_number_hash: &str,
        otp_hash: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<WhatsAppOtpRow> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO whatsapp_otps (id, user_id, phone_number_hash, otp_hash,
                                       created_at, expires_at, attempts)
            VALUES (?, ?, ?, ?, ?, ?, 0)
            "#,
        )
        .bind(&id)
        .bind(user_id)
        .bind(phone_number_hash)
        .bind(otp_hash)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(WhatsAppOtpRow {
            id,
            user_id: user_id.to_string(),
            phone_number_hash: phone_number_hash.to_string(),
            otp_hash: otp_hash.to_string(),
            created_at: now,
            expires_at,
            used_at: None,
            attempts: 0,
        })
    }

    /// Find latest OTP by phone hash.
    #[instrument(skip(self))]
    pub async fn find_latest_otp_by_phone_hash(
        &self,
        phone_number_hash: &str,
    ) -> Result<Option<WhatsAppOtpRow>> {
        let row = sqlx::query(
            r#"
            SELECT id, user_id, phone_number_hash, otp_hash, created_at, expires_at, used_at, attempts
            FROM whatsapp_otps
            WHERE phone_number_hash = ?
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(phone_number_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.map(|r| WhatsAppOtpRow {
            id: r.get("id"),
            user_id: r.get("user_id"),
            phone_number_hash: r.get("phone_number_hash"),
            otp_hash: r.get("otp_hash"),
            created_at: parse_datetime(&r.get::<String, _>("created_at")),
            expires_at: parse_datetime(&r.get::<String, _>("expires_at")),
            used_at: r.get::<Option<String>, _>("used_at").map(|s| parse_datetime(&s)),
            attempts: r.get("attempts"),
        }))
    }

    /// Increment OTP attempts.
    #[instrument(skip(self))]
    pub async fn increment_otp_attempts(&self, otp_id: &str) -> Result<i32> {
        sqlx::query("UPDATE whatsapp_otps SET attempts = attempts + 1 WHERE id = ?")
            .bind(otp_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        let row = sqlx::query("SELECT attempts FROM whatsapp_otps WHERE id = ?")
            .bind(otp_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(row.get("attempts"))
    }

    /// Mark OTP as used.
    #[instrument(skip(self))]
    pub async fn mark_otp_used(&self, otp_id: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query("UPDATE whatsapp_otps SET used_at = ? WHERE id = ?")
            .bind(now.to_rfc3339())
            .bind(otp_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;
        Ok(())
    }

    // =========================================================================
    // Group operations
    // =========================================================================

    /// List groups for a config.
    #[instrument(skip(self))]
    pub async fn list_groups(&self, config_id: &str) -> Result<Vec<WhatsAppGroupRow>> {
        let rows = sqlx::query(
            r#"
            SELECT id, config_id, name, description, color, is_default, created_at, updated_at
            FROM whatsapp_groups
            WHERE config_id = ?
            ORDER BY name
            "#,
        )
        .bind(config_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| WhatsAppGroupRow {
                id: r.get("id"),
                config_id: r.get("config_id"),
                name: r.get("name"),
                description: r.get("description"),
                color: r.get("color"),
                is_default: r.get::<i32, _>("is_default") == 1,
                created_at: parse_datetime(&r.get::<String, _>("created_at")),
                updated_at: parse_datetime(&r.get::<String, _>("updated_at")),
            })
            .collect())
    }

    /// Create a group.
    #[instrument(skip(self))]
    pub async fn create_group(
        &self,
        config_id: &str,
        name: &str,
        description: Option<&str>,
        color: Option<&str>,
    ) -> Result<WhatsAppGroupRow> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO whatsapp_groups (id, config_id, name, description, color,
                                         is_default, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, 0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(config_id)
        .bind(name)
        .bind(description)
        .bind(color)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| WhatsAppError::Config(e.to_string()))?;

        Ok(WhatsAppGroupRow {
            id,
            config_id: config_id.to_string(),
            name: name.to_string(),
            description: description.map(String::from),
            color: color.map(String::from),
            is_default: false,
            created_at: now,
            updated_at: now,
        })
    }

    /// Move conversation to a group.
    #[instrument(skip(self))]
    pub async fn move_conversation_to_group(
        &self,
        conversation_id: &str,
        group_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query("UPDATE whatsapp_conversations SET group_id = ? WHERE id = ?")
            .bind(group_id)
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;
        Ok(())
    }

    /// Delete a group.
    #[instrument(skip(self))]
    pub async fn delete_group(&self, group_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM whatsapp_groups WHERE id = ?")
            .bind(group_id)
            .execute(&self.pool)
            .await
            .map_err(|e| WhatsAppError::Config(e.to_string()))?;
        Ok(())
    }
}
