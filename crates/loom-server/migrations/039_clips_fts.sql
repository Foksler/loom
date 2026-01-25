-- Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
-- SPDX-License-Identifier: Proprietary

-- Full-text search virtual table for clips
-- Enables searching clips by name, description, owner, and language
CREATE VIRTUAL TABLE IF NOT EXISTS clips_fts USING fts5(
    clip_id UNINDEXED,
    owner,
    name,
    description,
    language,
    tokenize = 'unicode61',
    prefix = '2 3'
);

-- AFTER INSERT: Add to FTS
CREATE TRIGGER IF NOT EXISTS clips_fts_ai
AFTER INSERT ON clips
BEGIN
    INSERT INTO clips_fts (clip_id, owner, name, description, language)
    VALUES (
        new.id,
        COALESCE(new.owner, ''),
        COALESCE(new.name, ''),
        COALESCE(new.description, ''),
        COALESCE(new.language, '')
    );
END;

-- AFTER UPDATE: Replace in FTS
CREATE TRIGGER IF NOT EXISTS clips_fts_au
AFTER UPDATE ON clips
BEGIN
    DELETE FROM clips_fts WHERE clip_id = old.id;
    INSERT INTO clips_fts (clip_id, owner, name, description, language)
    VALUES (
        new.id,
        COALESCE(new.owner, ''),
        COALESCE(new.name, ''),
        COALESCE(new.description, ''),
        COALESCE(new.language, '')
    );
END;

-- AFTER DELETE: Remove from FTS
CREATE TRIGGER IF NOT EXISTS clips_fts_ad
AFTER DELETE ON clips
BEGIN
    DELETE FROM clips_fts WHERE clip_id = old.id;
END;

-- Backfill existing clips into FTS
INSERT INTO clips_fts (clip_id, owner, name, description, language)
SELECT
    id,
    COALESCE(owner, ''),
    COALESCE(name, ''),
    COALESCE(description, ''),
    COALESCE(language, '')
FROM clips;
