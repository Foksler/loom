-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- WhatsApp integration tables

-- Per-org WhatsApp configuration
CREATE TABLE whatsapp_configs (
    id TEXT PRIMARY KEY,
    org_id TEXT NOT NULL UNIQUE REFERENCES organizations(id) ON DELETE CASCADE,
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
    config_id TEXT NOT NULL REFERENCES whatsapp_configs(id) ON DELETE CASCADE,
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
    config_id TEXT NOT NULL REFERENCES whatsapp_configs(id) ON DELETE CASCADE,
    group_id TEXT REFERENCES whatsapp_groups(id) ON DELETE SET NULL,
    wa_phone_number TEXT NOT NULL,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    thread_id TEXT REFERENCES threads(id) ON DELETE SET NULL,
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
CREATE INDEX idx_whatsapp_conversations_thread ON whatsapp_conversations(thread_id);

-- Message history
CREATE TABLE whatsapp_messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES whatsapp_conversations(id) ON DELETE CASCADE,
    wa_message_id TEXT NOT NULL UNIQUE,
    direction TEXT NOT NULL,
    message_type TEXT NOT NULL,
    content TEXT,
    media_url TEXT,
    status TEXT DEFAULT 'pending',
    timestamp TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX idx_whatsapp_messages_conversation ON whatsapp_messages(conversation_id);
CREATE INDEX idx_whatsapp_messages_wa_id ON whatsapp_messages(wa_message_id);
CREATE INDEX idx_whatsapp_messages_timestamp ON whatsapp_messages(timestamp);

-- OTP verification codes for phone linking
CREATE TABLE whatsapp_otps (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    phone_number_hash TEXT NOT NULL,
    otp_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    attempts INTEGER DEFAULT 0
);

CREATE INDEX idx_whatsapp_otps_phone ON whatsapp_otps(phone_number_hash);
CREATE INDEX idx_whatsapp_otps_user ON whatsapp_otps(user_id);
CREATE INDEX idx_whatsapp_otps_expires ON whatsapp_otps(expires_at);
