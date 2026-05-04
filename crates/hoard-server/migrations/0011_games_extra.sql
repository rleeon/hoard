-- Phase 2: enrich the games catalog so the desktop client can do auto-detection
-- and so we can re-import the Ludusavi manifest non-destructively.
--
-- Existing rows (the 10 hand-seeded games) keep their data; new columns are
-- nullable or have sensible defaults.

ALTER TABLE games ADD COLUMN notes TEXT;
ALTER TABLE games ADD COLUMN cloud_steam INTEGER NOT NULL DEFAULT 0;
ALTER TABLE games ADD COLUMN cloud_gog INTEGER NOT NULL DEFAULT 0;
ALTER TABLE games ADD COLUMN steam_app_id INTEGER;
-- Provenance: 'manual' for hand-added games, 'ludusavi-manifest' for imported.
ALTER TABLE games ADD COLUMN imported_from TEXT NOT NULL DEFAULT 'manual';
-- Version of the source manifest at the time of (re-)import.
ALTER TABLE games ADD COLUMN manifest_version TEXT;
-- When a row was last touched by an import (NULL for hand-added).
ALTER TABLE games ADD COLUMN imported_at TEXT;

CREATE INDEX IF NOT EXISTS idx_games_imported_from ON games(imported_from);
CREATE INDEX IF NOT EXISTS idx_games_steam_app_id ON games(steam_app_id);

-- Track when the manifest was last imported, so the client can show "catalog
-- as of <date>" and decide whether to suggest a refresh.
CREATE TABLE IF NOT EXISTS manifest_imports (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    source          TEXT NOT NULL,                  -- URL or file path
    manifest_version TEXT,                          -- as reported by the file, if any
    games_inserted  INTEGER NOT NULL DEFAULT 0,
    games_updated   INTEGER NOT NULL DEFAULT 0,
    games_pruned    INTEGER NOT NULL DEFAULT 0,
    imported_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
