-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- Add key_prefix column to crash_api_keys table for display purposes
ALTER TABLE crash_api_keys ADD COLUMN key_prefix TEXT;

-- Create a system user for internal/automated operations
-- This user is used for system-generated API keys and other automated tasks
INSERT OR IGNORE INTO users (id, display_name, primary_email, created_at, updated_at)
VALUES ('00000000-0000-0000-0000-000000000000', 'System', 'system@loom.internal', datetime('now'), datetime('now'));
