-- Content-defined chunk store (ADR 0019, Fase 4).
--
-- Extends the whole-file blob store (ADR 0018, migration 0013). A *large*
-- file (> CHUNK_THRESHOLD, see chunking.rs) is no longer stored as one blob:
-- it is split by an in-house content-defined chunker into variable-size
-- pieces, each addressed by its own sha256 and stored once per user at
-- chunks/<user_id>/<sha[0:2]>/<sha256>. A monolithic save that rewrites a few
-- KB per version then re-uploads only the chunks that actually changed.
--
-- Small files keep the blob path untouched (0013): a given content is either
-- always a blob (size <= threshold) or always chunked (size > threshold),
-- never both — the threshold is a pure function of size, which is a function
-- of content, so the two stores never overlap for the same bytes.
--
-- refcount semantics mirror blobs: it counts EVERY referencing
-- snapshot_file_chunks row (live or trashed). GC of the chunk file + this row
-- happens only when refcount reaches 0 during the trash purge.
CREATE TABLE IF NOT EXISTS chunks (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sha256     TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    refcount   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
    PRIMARY KEY (user_id, sha256)
);

-- Ordered list of chunks that make up one chunked snapshot_files row. The
-- whole-file sha256 still lives on snapshot_files (integrity / metadata); the
-- bytes are reassembled by concatenating these chunks in `ordinal` order.
-- ON DELETE CASCADE off snapshot_files means purging a snapshot drops its
-- chunk-reference rows automatically (the trash purge reads them first to
-- decrement chunk refcounts).
CREATE TABLE IF NOT EXISTS snapshot_file_chunks (
    snapshot_file_id TEXT NOT NULL REFERENCES snapshot_files(id) ON DELETE CASCADE,
    ordinal          INTEGER NOT NULL,
    chunk_sha256     TEXT NOT NULL,
    PRIMARY KEY (snapshot_file_id, ordinal)
);

CREATE INDEX IF NOT EXISTS idx_sfc_file ON snapshot_file_chunks(snapshot_file_id);
