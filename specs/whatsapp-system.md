<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# WhatsApp Integration System Specification

**Status:** Implemented\
**Version:** 1.0\
**Last Updated:** 2026-01-25

---

## 1. Overview

### Purpose

Integrate WhatsApp Business Cloud API with loom-server to enable users to interact with Loom AI agents via WhatsApp messaging. Users can send messages from WhatsApp, which are stored as threads and optionally trigger LLM responses.

### Goals

- **Webhook integration** with WhatsApp Business Cloud API v21.0
- **Message persistence** linking WhatsApp conversations to Loom threads
- **Phone linking** for user identity verification via OTP
- **Conversation grouping** to organize messages by topic
- **24-hour window** compliance for session-based messaging
- **Analytics integration** following loom-web tracking patterns

### Non-Goals

- Template message management (Phase 2)
- Voice message transcription (Phase 2)
- WhatsApp Business API on-premise (Cloud API only)
- Group chats (1:1 conversations only)

---

## 2. Architecture

### Component Diagram

```
┌─────────────┐     HTTPS      ┌─────────────┐     Cloud API    ┌─────────────┐
│  WhatsApp   │ ──────────────▶│ loom-server │ ────────────────▶│   Meta      │
│   User      │  Webhooks      │             │   Send messages  │  WhatsApp   │
└─────────────┘                │  /api/wa/*  │                  │    API      │
                               └──────┬──────┘                  └─────────────┘
                                      │
                                      ▼
                               ┌─────────────┐
                               │   SQLite    │
                               │  (configs,  │
                               │  messages)  │
                               └─────────────┘
```

### Crate Structure

```
crates/
├── loom-whatsapp/              # Shared types & client
│   ├── src/
│   │   ├── lib.rs              # Re-exports
│   │   ├── client.rs           # WhatsApp Cloud API client
│   │   ├── config.rs           # WhatsAppConfig with SecretString
│   │   ├── error.rs            # WhatsAppError enum
│   │   ├── types.rs            # Webhook payloads, message types
│   │   └── webhook.rs          # Signature verification
│   └── Cargo.toml
├── loom-server-whatsapp/       # Server-side integration
│   ├── src/
│   │   ├── lib.rs              # Re-exports
│   │   ├── repository.rs       # SQLite storage
│   │   ├── service.rs          # Message processing orchestration
│   │   └── conversation.rs     # 24-hour window tracking
│   └── Cargo.toml
```

---

## 3. Conversation Grouping Hierarchy

WhatsApp messages integrate with Loom's organizational hierarchy:

```
Organization (Loom parent)
  └── WhatsApp Config (per org)
        └── Groups/Topics (optional categorization)
              └── Conversations (per phone number)
                    └── Thread (linked 1:1)
                          └── Messages
```

### Group Model

Organizations can create groups/topics for categorizing conversations:
- Examples: "Support", "Sales", "Engineering", "General"
- Conversations assigned to groups manually or via rules
- Default group for new conversations

---

## 4. Database Schema

Migration: `crates/loom-server/migrations/037_whatsapp.sql`

```sql
-- Per-org WhatsApp configuration
CREATE TABLE whatsapp_configs (
    id TEXT PRIMARY KEY,
    org_id TEXT NOT NULL UNIQUE REFERENCES organizations(id),
    phone_number_id TEXT NOT NULL,
    access_token_encrypted TEXT NOT NULL,
    app_secret_hash TEXT NOT NULL,
    verify_token_hash TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_whatsapp_configs_org ON whatsapp_configs(org_id);
CREATE INDEX idx_whatsapp_configs_phone ON whatsapp_configs(phone_number_id);

-- WhatsApp conversation groups (topics)
CREATE TABLE whatsapp_groups (
    id TEXT PRIMARY KEY,
    config_id TEXT NOT NULL REFERENCES whatsapp_configs(id),
    name TEXT NOT NULL,
    description TEXT,
    color TEXT,
    is_default INTEGER DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(config_id, name)
);

CREATE INDEX idx_whatsapp_groups_config ON whatsapp_groups(config_id);

-- Conversation tracking (24-hour windows)
CREATE TABLE whatsapp_conversations (
    id TEXT PRIMARY KEY,
    config_id TEXT NOT NULL REFERENCES whatsapp_configs(id),
    group_id TEXT REFERENCES whatsapp_groups(id),
    wa_phone_number TEXT NOT NULL,
    user_id TEXT REFERENCES users(id),
    thread_id TEXT REFERENCES threads(id),
    last_customer_message_at TEXT NOT NULL,
    session_expires_at TEXT NOT NULL,
    status TEXT DEFAULT 'active',
    created_at TEXT NOT NULL,
    UNIQUE(config_id, wa_phone_number)
);

CREATE INDEX idx_whatsapp_conversations_config ON whatsapp_conversations(config_id);
CREATE INDEX idx_whatsapp_conversations_group ON whatsapp_conversations(group_id);
CREATE INDEX idx_whatsapp_conversations_phone ON whatsapp_conversations(wa_phone_number);
CREATE INDEX idx_whatsapp_conversations_user ON whatsapp_conversations(user_id);

-- Message history
CREATE TABLE whatsapp_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES whatsapp_conversations(id),
    wa_message_id TEXT NOT NULL UNIQUE,
    direction TEXT NOT NULL,
    message_type TEXT NOT NULL,
    content TEXT,
    status TEXT DEFAULT 'pending',
    timestamp TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_whatsapp_messages_conversation ON whatsapp_messages(conversation_id);
CREATE INDEX idx_whatsapp_messages_wa_id ON whatsapp_messages(wa_message_id);

-- OTP verification codes
CREATE TABLE whatsapp_otps (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id),
    phone_number_hash TEXT NOT NULL,
    otp_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    attempts INTEGER DEFAULT 0
);

CREATE INDEX idx_whatsapp_otps_phone ON whatsapp_otps(phone_number_hash);
CREATE INDEX idx_whatsapp_otps_user ON whatsapp_otps(user_id);
```

---

## 5. API Endpoints

### 5.1 Public (Webhook)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/whatsapp/webhook` | Challenge-response verification |
| POST | `/api/whatsapp/webhook` | Incoming messages/statuses |

### 5.2 Authenticated (Org Config)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/orgs/{org_id}/whatsapp/config` | Create/update config |
| GET | `/api/orgs/{org_id}/whatsapp/config` | Get config status |
| DELETE | `/api/orgs/{org_id}/whatsapp/config` | Remove config |

### 5.3 Authenticated (Groups)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/orgs/{org_id}/whatsapp/groups` | List groups |
| POST | `/api/orgs/{org_id}/whatsapp/groups` | Create group |
| PATCH | `/api/orgs/{org_id}/whatsapp/groups/{id}` | Update group |
| DELETE | `/api/orgs/{org_id}/whatsapp/groups/{id}` | Delete group |
| POST | `/api/orgs/{org_id}/whatsapp/conversations/{id}/move` | Move to group |

### 5.4 Authenticated (Conversations)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/orgs/{org_id}/whatsapp/conversations` | List conversations |
| GET | `/api/orgs/{org_id}/whatsapp/conversations/{id}` | Get conversation |

### 5.5 Authenticated (User Phone Linking)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/users/me/whatsapp/link` | Initiate phone linking |
| POST | `/api/users/me/whatsapp/verify` | Complete verification |
| DELETE | `/api/users/me/whatsapp/unlink` | Unlink phone |

---

## 6. Core Types

### 6.1 Provider Extension

File: `crates/loom-server-auth/src/user.rs`

```rust
pub enum Provider {
    GitHub,
    Google,
    MagicLink,
    WhatsApp,  // NEW
}
```

### 6.2 WhatsApp Types

File: `crates/loom-whatsapp/src/types.rs`

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Webhook verification query params
#[derive(Debug, Deserialize)]
pub struct WebhookVerifyParams {
    #[serde(rename = "hub.mode")]
    pub mode: String,
    #[serde(rename = "hub.verify_token")]
    pub verify_token: String,
    #[serde(rename = "hub.challenge")]
    pub challenge: String,
}

/// Incoming webhook payload
#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub object: String,
    pub entry: Vec<WebhookEntry>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookEntry {
    pub id: String,
    pub changes: Vec<WebhookChange>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookChange {
    pub value: WebhookValue,
    pub field: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhookValue {
    pub messaging_product: String,
    pub metadata: WebhookMetadata,
    #[serde(default)]
    pub messages: Vec<InboundMessage>,
    #[serde(default)]
    pub statuses: Vec<MessageStatus>,
}

#[derive(Debug, Deserialize)]
pub struct WebhookMetadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Deserialize)]
pub struct InboundMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(default)]
    pub text: Option<TextMessage>,
    #[serde(default)]
    pub image: Option<MediaMessage>,
    #[serde(default)]
    pub document: Option<MediaMessage>,
}

#[derive(Debug, Deserialize)]
pub struct TextMessage {
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct MediaMessage {
    pub id: String,
    pub mime_type: String,
    #[serde(default)]
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageStatus {
    pub id: String,
    pub status: String,
    pub timestamp: String,
    pub recipient_id: String,
}

/// Outbound message request
#[derive(Debug, Serialize)]
pub struct SendMessageRequest {
    pub messaging_product: String,
    pub recipient_type: String,
    pub to: String,
    #[serde(rename = "type")]
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<OutboundText>,
}

#[derive(Debug, Serialize)]
pub struct OutboundText {
    pub preview_url: bool,
    pub body: String,
}

/// Send message response
#[derive(Debug, Deserialize)]
pub struct SendMessageResponse {
    pub messaging_product: String,
    pub contacts: Vec<Contact>,
    pub messages: Vec<SentMessage>,
}

#[derive(Debug, Deserialize)]
pub struct Contact {
    pub input: String,
    pub wa_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SentMessage {
    pub id: String,
}
```

### 6.3 Error Types

File: `crates/loom-whatsapp/src/error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WhatsAppError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Request timed out")]
    Timeout,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("WhatsApp API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Invalid webhook signature")]
    InvalidWebhookSignature,

    #[error("Invalid challenge verification")]
    InvalidChallenge,

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Session expired - customer must message first")]
    SessionExpired,

    #[error("Phone verification failed: {0}")]
    VerificationFailed(String),
}

impl loom_http::RetryableError for WhatsAppError {
    fn is_retryable(&self) -> bool {
        matches!(self,
            Self::Network(_) | Self::Timeout | Self::RateLimited |
            Self::ApiError { status, .. } if *status >= 500
        )
    }
}
```

---

## 7. Webhook Implementation

### 7.1 Challenge Verification (GET)

```rust
pub async fn whatsapp_webhook_verify(
    Query(params): Query<WebhookVerifyParams>,
    State(state): State<AppState>,
) -> Result<String, ServerError> {
    // 1. Verify hub.mode == "subscribe"
    if params.mode != "subscribe" {
        return Err(ServerError::BadRequest("Invalid mode".into()));
    }

    // 2. Find config by verify_token (check all orgs)
    let config = state.whatsapp_repo
        .find_by_verify_token_hash(&hash_token(&params.verify_token))
        .await?
        .ok_or(ServerError::Unauthorized)?;

    // 3. Return hub.challenge as plain text
    Ok(params.challenge)
}
```

### 7.2 Webhook Handler (POST)

```rust
pub async fn whatsapp_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, ServerError> {
    // 1. Extract X-Hub-Signature-256 header
    let signature = headers
        .get("X-Hub-Signature-256")
        .and_then(|v| v.to_str().ok())
        .ok_or(ServerError::BadRequest("Missing signature".into()))?;

    // 2. Parse payload to get phone_number_id
    let payload: WebhookPayload = serde_json::from_slice(&body)?;
    let phone_number_id = payload.entry.first()
        .and_then(|e| e.changes.first())
        .map(|c| &c.value.metadata.phone_number_id)
        .ok_or(ServerError::BadRequest("Invalid payload".into()))?;

    // 3. Find config and verify signature
    let config = state.whatsapp_repo
        .find_by_phone_number_id(phone_number_id)
        .await?
        .ok_or(ServerError::NotFound)?;

    verify_webhook_signature(&config.app_secret, signature, &body)?;

    // 4. Process messages asynchronously
    let service = state.whatsapp_service.clone();
    tokio::spawn(async move {
        if let Err(e) = service.process_webhook(payload).await {
            tracing::error!(error = %e, "Failed to process WhatsApp webhook");
        }
    });

    // 5. Return 200 OK immediately
    Ok(StatusCode::OK)
}
```

### 7.3 Signature Verification

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn verify_webhook_signature(
    app_secret: &str,
    signature_header: &str,
    body: &[u8],
) -> Result<(), WhatsAppError> {
    let expected_prefix = "sha256=";
    if !signature_header.starts_with(expected_prefix) {
        return Err(WhatsAppError::InvalidWebhookSignature);
    }

    let expected_sig = hex::decode(&signature_header[expected_prefix.len()..])?;

    let mut mac = Hmac::<Sha256>::new_from_slice(app_secret.as_bytes())?;
    mac.update(body);

    // Constant-time comparison
    mac.verify_slice(&expected_sig)
        .map_err(|_| WhatsAppError::InvalidWebhookSignature)
}
```

---

## 8. Message Flow

```
WhatsApp Webhook (POST /api/whatsapp/webhook)
    │
    ├─ 1. Verify X-Hub-Signature-256 (HMAC-SHA256)
    │
    ├─ 2. Parse webhook payload
    │     └─ Extract: phone_number_id, from, message_id, type, content
    │
    ├─ 3. Find WhatsAppConfig by phone_number_id
    │
    ├─ 4. Find/Create WhatsAppConversation
    │     ├─ Key: (config_id, wa_phone_number)
    │     └─ Update: last_customer_message_at, session_expires_at (+24h)
    │
    ├─ 5. Find linked User (optional)
    │     └─ Query: identities WHERE provider='whatsapp' AND provider_user_id={phone}
    │
    ├─ 6. Find/Create Thread
    │     ├─ Thread ID: Derive from conversation_id or create new
    │     ├─ Set: metadata.source = "whatsapp"
    │     ├─ Set: metadata.extra.phone = {wa_phone_number}
    │     └─ Set: metadata.extra.group_id = {group_id}
    │
    ├─ 7. Add Message to Thread
    │     └─ MessageSnapshot { role: User, content: message_text }
    │
    ├─ 8. Return 200 OK immediately (async processing below)
    │
    └─ 9. (Background - Phase 5) Trigger LLM Response
          ├─ LlmService.complete_streaming_anthropic(request)
          ├─ Accumulate LlmEvent::TextDelta chunks
          ├─ On LlmEvent::Completed:
          │     ├─ Add Assistant message to Thread
          │     ├─ Check session_expires_at > now
          │     └─ If within window: WhatsAppClient.send_text()
          └─ Handle errors, log failures
```

---

## 9. Phone Verification Flow

```
1. User clicks "Link WhatsApp" in Loom UI
     │
2. POST /api/users/me/whatsapp/link { phone_number: "+1234567890" }
     │
3. Server generates 6-digit OTP
     ├─ Store: Argon2id(otp) in whatsapp_otps
     ├─ Store: SHA-256(phone) for lookup
     └─ Set: expires_at = now + 5 minutes
     │
4. Server sends OTP via WhatsApp
     └─ WhatsAppClient.send_text(phone, "Your Loom code: 123456")
     │
5. User enters code in Loom UI
     │
6. POST /api/users/me/whatsapp/verify { phone_number, otp }
     │
7. Server validates:
     ├─ Find OTP by SHA-256(phone)
     ├─ Check: attempts < 3
     ├─ Check: expires_at > now
     ├─ Check: used_at IS NULL
     └─ Verify: Argon2id(otp) matches stored hash
     │
8. On success:
     ├─ Mark OTP as used (used_at = now)
     └─ Create Identity { provider: WhatsApp, provider_user_id: phone }
```

---

## 10. 24-Hour Window

WhatsApp enforces a 24-hour messaging window:

- **Within window**: Send text/media freely after customer message
- **Outside window**: Require pre-approved template messages
- **MVP approach**: Store responses, show "session expired" in UI

```rust
impl WhatsAppService {
    pub fn is_session_active(&self, conversation: &WhatsAppConversation) -> bool {
        conversation.session_expires_at > Utc::now()
    }

    pub async fn send_response(
        &self,
        conversation: &WhatsAppConversation,
        text: &str,
    ) -> Result<(), WhatsAppError> {
        if !self.is_session_active(conversation) {
            return Err(WhatsAppError::SessionExpired);
        }
        self.client.send_text(&conversation.wa_phone_number, text).await
    }
}
```

---

## 11. Security

### 11.1 Secret Storage

Following `loom-server-secrets` patterns:

```rust
pub struct WhatsAppConfig {
    pub id: Uuid,
    pub org_id: OrgId,
    pub phone_number_id: String,

    // Encrypted using envelope encryption (AES-256-GCM)
    pub access_token: SecretString,

    // Hashed for lookup/verification (Argon2id)
    pub app_secret_hash: String,
    pub verify_token_hash: String,

    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### 11.2 Security Rate Limiting

Only for security-sensitive operations:

| Limit | Value | Enforcement |
|-------|-------|-------------|
| OTP attempts | 3 max per phone | Counter in `whatsapp_otps.attempts` |
| OTP expiry | 5 minutes | `expires_at` column check |
| OTP cooldown | 1 minute between requests | Check `created_at` of latest OTP |
| Failed verifications | 5 per hour per IP | In-memory counter |

---

## 12. Analytics Integration

WhatsApp UI pages integrate with `$lib/analytics`:

### 12.1 Tracking Events

| User Action | Event | Properties |
|-------------|-------|------------|
| Save config | `form_submitted` | `form_name: 'whatsapp_config'`, `org_id` |
| Delete config | `action_performed` | `action: 'delete'`, `resource_type: 'whatsapp_config'` |
| Request OTP | `button_clicked` | `button_name: 'request_whatsapp_otp'` |
| Verify phone | `form_submitted` | `form_name: 'whatsapp_verify'` |
| Link success | `action_performed` | `action: 'link'`, `resource_type: 'whatsapp_phone'` |
| Create group | `action_performed` | `action: 'create'`, `resource_type: 'whatsapp_group'` |
| Move conversation | `action_performed` | `action: 'move'`, `resource_type: 'whatsapp_conversation'` |
| Filter by group | `filter_changed` | `filter_name: 'whatsapp_group'`, `filter_value` |

---

## 13. Health Check

Add to `crates/loom-server/src/health.rs`:

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct WhatsAppHealth {
    pub status: HealthStatus,
    pub latency_ms: u64,
    pub configured: bool,
    pub configs_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn check_whatsapp(
    repo: Option<Arc<WhatsAppRepository>>,
) -> WhatsAppHealth {
    let start = Instant::now();

    let (configured, status, configs_count, error) = match repo {
        None => (false, HealthStatus::Degraded, 0, Some("WhatsApp not configured".into())),
        Some(repo) => {
            match timeout(Duration::from_secs(5), repo.count_enabled_configs()).await {
                Ok(Ok(count)) => (true, HealthStatus::Healthy, count, None),
                Ok(Err(e)) => (true, HealthStatus::Degraded, 0, Some(e.to_string())),
                Err(_) => (true, HealthStatus::Degraded, 0, Some("Timeout".into())),
            }
        }
    };

    WhatsAppHealth {
        status,
        latency_ms: start.elapsed().as_millis() as u64,
        configured,
        configs_count,
        error,
    }
}
```

---

## 14. i18n Keys

Add to `crates/loom-i18n/locales/{locale}/messages.po`:

```gettext
msgid "server.api.whatsapp.not_configured"
msgstr "WhatsApp integration is not configured"

msgid "server.api.whatsapp.invalid_signature"
msgstr "Invalid webhook signature"

msgid "server.api.whatsapp.invalid_phone"
msgstr "Invalid phone number format (must be E.164)"

msgid "server.api.whatsapp.otp_expired"
msgstr "Verification code has expired"

msgid "server.api.whatsapp.otp_invalid"
msgstr "Invalid verification code"

msgid "server.api.whatsapp.otp_too_many_attempts"
msgstr "Too many failed attempts. Please request a new code."

msgid "server.api.whatsapp.phone_already_linked"
msgstr "This phone number is already linked to an account"

msgid "server.api.whatsapp.session_expired"
msgstr "Session expired. Customer must send a message first."
```

---

## 15. Environment Variables

```bash
LOOM_SERVER_WHATSAPP_BASE_URL=https://graph.facebook.com/v21.0
LOOM_SERVER_WHATSAPP_TIMEOUT_SECS=30
```

---

## 16. Files to Modify

### Backend (Rust)

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `loom-whatsapp`, `loom-server-whatsapp` |
| `crates/loom-server/Cargo.toml` | Add dependencies |
| `crates/loom-server/migrations/037_whatsapp.sql` | New migration |
| `crates/loom-server-auth/src/user.rs` | Add `Provider::WhatsApp` |
| `crates/loom-server/src/api.rs` | Add repos/services to AppState |
| `crates/loom-server/src/routes/mod.rs` | Add `pub mod whatsapp;` |
| `crates/loom-server/src/routes/whatsapp/` | New route handlers |
| `crates/loom-server/src/health.rs` | Add `WhatsAppHealth` |
| `crates/loom-i18n/locales/*/messages.po` | Add i18n keys |

### Frontend (Svelte)

| File | Change |
|------|--------|
| `web/loom-web/src/routes/(app)/settings/orgs/[orgId]/whatsapp/+page.svelte` | Config page |
| `web/loom-web/src/routes/(app)/settings/orgs/[orgId]/whatsapp/groups/+page.svelte` | Groups page |
| `web/loom-web/src/routes/(app)/settings/profile/whatsapp/+page.svelte` | Phone linking |
| `web/loom-web/src/routes/(app)/whatsapp/+page.svelte` | Conversations list |
| `web/loom-web/src/routes/(app)/whatsapp/[convId]/+page.svelte` | Conversation detail |
| `web/loom-web/src/lib/api/client.ts` | API methods |

---

## 17. Implementation Phases

### Phase 1: Foundation
- Create `loom-whatsapp` crate (types, config, error)
- Create `loom-server-whatsapp` crate (repository skeleton)
- Add migration `037_whatsapp.sql`
- Add `Provider::WhatsApp` variant
- Run `cargo2nix-update`

### Phase 2: Webhooks
- Implement signature verification
- Implement challenge verification
- Add webhook routes (GET/POST)
- Add to PublicRouter

### Phase 3: Message Storage & Threads
- Implement `WhatsAppClient` for sending messages
- Implement `WhatsAppService` for message storage
- Implement conversation tracking (24h windows)
- Link conversations to threads

### Phase 4: User Integration & Phone Linking
- Implement WhatsApp OTP verification flow
- Add phone linking endpoints
- Add org config management endpoints
- Integrate with identity system

### Phase 5: Conversation Grouping
- Implement group CRUD
- Add group assignment to conversations
- Build Group Management UI

### Phase 6: Full Agent Mode (Optional)
- Enable automatic LLM responses
- Add 24-hour window enforcement
- Handle long responses (split at 4096 chars)

### Phase 7: Testing & Hardening
- Unit tests (signature, types, parsing)
- Integration tests (webhook flow)
- Authorization tests
- Property-based tests

---

## 18. Testing

```bash
# Run tests
cargo test -p loom-whatsapp
cargo test -p loom-server-whatsapp
cargo test -p loom-server authz_whatsapp

# Build check
cargo build --workspace
cargo clippy --workspace -- -D warnings
```

---

## Appendix A: WhatsApp API Reference

- **API Version:** v21.0
- **Base URL:** `https://graph.facebook.com/v21.0`
- **Send Message:** `POST /{phone_number_id}/messages`
- **Media Download:** `GET /{media_id}` (URLs expire in 5 minutes)

### Message Limits

| Type | Max Size | Formats |
|------|----------|---------|
| Text | 4096 chars | Plain text |
| Images | 5 MB | JPG, PNG |
| Documents | 100 MB | PDF, DOCX, XLSX |
| Audio | 16 MB | AAC, MP3, OGG |
