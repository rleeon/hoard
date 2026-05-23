-- Saves: one logical save slot per (user, game, label). Matches the
-- self-hosted SQLite schema shape so the desktop client can speak the same
-- /v1/saves protocol against either backend.

CREATE TABLE IF NOT EXISTS saves (
    id                  TEXT PRIMARY KEY,
    user_id             UUID NOT NULL REFERENCES profiles(user_id) ON DELETE CASCADE,
    game_slug           TEXT NOT NULL,
    label               TEXT NOT NULL DEFAULT 'default',
    local_path_hint     TEXT,
    client_os           TEXT,
    latest_version_num  BIGINT NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, game_slug, label)
);

CREATE INDEX IF NOT EXISTS idx_saves_user_game ON saves(user_id, game_slug);
