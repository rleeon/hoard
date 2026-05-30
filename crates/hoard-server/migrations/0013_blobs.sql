-- Content-addressed blob store (ADR 0018, eje C).
-- Each unique file (by sha256) is stored once per user on disk at
-- blobs/<user_id>/<sha[0:2]>/<sha256>; this table tracks its size and how
-- many snapshot_files rows reference it. Dedup is per-user (the user_id in
-- the key) so content existence never leaks across accounts.
--
-- refcount counts EVERY referencing snapshot_files row, including rows of
-- soft-deleted (trashed) snapshots — a trashed snapshot still pins its bytes
-- on disk and against quota until it is purged. GC (delete the blob file and
-- this row) happens only when refcount reaches 0 during the trash purge.
CREATE TABLE IF NOT EXISTS blobs (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sha256     TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    refcount   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (user_id, sha256)
);
