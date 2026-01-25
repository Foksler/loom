-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- Clips (code snippets) table
-- Each clip is stored as a git bare repository for version control

CREATE TABLE IF NOT EXISTS clips (
    id TEXT PRIMARY KEY,
    -- Owner can be a username or org name
    owner TEXT NOT NULL,
    -- Clip name (unique per owner)
    name TEXT NOT NULL,
    -- Optional description
    description TEXT,
    -- Visibility: private, internal, public
    visibility TEXT NOT NULL DEFAULT 'private',
    -- User who created the clip
    created_by TEXT NOT NULL,
    -- Optional org that owns the clip (NULL for personal clips)
    org_id TEXT,
    -- Fork information
    is_fork INTEGER NOT NULL DEFAULT 0,
    forked_from TEXT,
    -- Statistics
    file_count INTEGER NOT NULL DEFAULT 0,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    -- Primary language (detected from files)
    language TEXT,
    -- Timestamps
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    -- Unique constraint on owner + name
    UNIQUE(owner, name),

    -- Foreign keys
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (org_id) REFERENCES organizations(id),
    FOREIGN KEY (forked_from) REFERENCES clips(id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_clips_owner ON clips(owner);
CREATE INDEX IF NOT EXISTS idx_clips_created_by ON clips(created_by);
CREATE INDEX IF NOT EXISTS idx_clips_org_id ON clips(org_id);
CREATE INDEX IF NOT EXISTS idx_clips_visibility ON clips(visibility);
CREATE INDEX IF NOT EXISTS idx_clips_language ON clips(language);
CREATE INDEX IF NOT EXISTS idx_clips_updated_at ON clips(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_clips_forked_from ON clips(forked_from);
