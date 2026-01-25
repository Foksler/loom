-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- Clip stars table for tracking user favorites

CREATE TABLE IF NOT EXISTS clip_stars (
    clip_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    created_at TEXT NOT NULL,

    PRIMARY KEY (clip_id, user_id),
    FOREIGN KEY (clip_id) REFERENCES clips(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_clip_stars_clip_id ON clip_stars(clip_id);
CREATE INDEX IF NOT EXISTS idx_clip_stars_user_id ON clip_stars(user_id);
CREATE INDEX IF NOT EXISTS idx_clip_stars_created_at ON clip_stars(created_at DESC);

-- Add star count column to clips table
ALTER TABLE clips ADD COLUMN star_count INTEGER NOT NULL DEFAULT 0;
