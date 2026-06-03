//! In-house content-defined chunking (ADR 0019, Fase 4).
//!
//! A monolithic save (one big file rewritten in place each play session) is
//! pathological for the whole-file blob store: a few changed KB invalidate the
//! whole-file sha and force re-storing the entire file every version. Splitting
//! the file at *content-defined* boundaries means a small edit only shifts the
//! chunks around the edit; everything else keeps its sha and dedups against the
//! previous upload.
//!
//! The boundary detector is a gear-hash rolling hash (the core of FastCDC)
//! implemented here rather than pulled from a crate — it's ~40 lines and avoids
//! a dependency on file-format-stable chunk boundaries we'd have to pin anyway.
//! Boundaries depend only on the byte content within the rolling window, so two
//! uploads that share a run of bytes produce identical chunks over that run.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Files at or below this are stored as a single whole-file blob (ADR 0018).
/// Above it, the file is chunked. A pure function of size keeps the blob and
/// chunk stores from ever holding the same content twice.
pub const CHUNK_THRESHOLD: u64 = 128 * 1024 * 1024;

/// Minimum chunk size — below this we never cut, so a degenerate input can't
/// produce a flood of tiny chunks.
const MIN_CHUNK: usize = 1 << 20; // 1 MiB
/// Hard cap so a stretch with no natural boundary still terminates.
const MAX_CHUNK: usize = 4 << 20; // 4 MiB
/// A boundary is declared when the low bits of the rolling hash are zero.
/// 20 set bits → ~1-in-1MiB cut probability on top of MIN_CHUNK ⇒ ~2 MiB avg.
const MASK: u64 = (1 << 20) - 1;

/// One chunk's placement plan: its content address, byte length, and where it
/// starts in the source file (so the commit phase can re-read exactly that
/// range without re-running the chunker).
#[derive(Debug, Clone)]
pub struct ChunkPlan {
    pub sha256: String,
    pub offset: u64,
    pub len: usize,
}

/// Absolute on-disk path of a chunk, sharded like blobs by the first two hex
/// chars of the sha. Deliberately a sibling of `blobs/` under the same
/// `data_dir` so chunk placement can use `rename` on the same filesystem.
pub fn chunk_path(data_dir: &Path, user_id: &str, sha256: &str) -> PathBuf {
    let prefix = if sha256.len() >= 2 {
        &sha256[..2]
    } else {
        "00"
    };
    data_dir
        .join("chunks")
        .join(user_id)
        .join(prefix)
        .join(sha256)
}

/// Gear table: 256 pseudo-random u64 built deterministically (splitmix64) at
/// compile time, so the boundary function is fixed across builds — a chunk
/// written by one server version dedups against the same content from another.
const GEAR: [u64; 256] = build_gear();

const fn build_gear() -> [u64; 256] {
    let mut t = [0u64; 256];
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut i = 0;
    while i < 256 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        t[i] = z;
        i += 1;
    }
    t
}

/// Stream `src` through the gear-hash chunker, hashing each chunk's bytes, and
/// return the ordered list of [`ChunkPlan`]s. Reads in fixed-size blocks; no
/// more than `MAX_CHUNK` bytes of the file are buffered at once. Does not touch
/// the DB or place anything on disk — that's the commit phase's job, so a quota
/// rejection costs only the read, not a rollback.
pub async fn plan_chunks(src: &Path) -> std::io::Result<Vec<ChunkPlan>> {
    let mut file = tokio::fs::File::open(src).await?;
    let mut plans: Vec<ChunkPlan> = Vec::new();

    let mut read_buf = vec![0u8; 256 * 1024];
    // Bytes of the current chunk, hashed in bulk at the boundary (per-byte
    // Sha256 calls would dominate the runtime on a multi-GB file).
    let mut pending: Vec<u8> = Vec::with_capacity(MAX_CHUNK);
    let mut gear: u64 = 0;
    let mut chunk_start: u64 = 0;

    loop {
        let n = file.read(&mut read_buf).await?;
        if n == 0 {
            break;
        }
        for &b in &read_buf[..n] {
            pending.push(b);
            gear = (gear << 1).wrapping_add(GEAR[b as usize]);
            let boundary =
                (pending.len() >= MIN_CHUNK && (gear & MASK) == 0) || pending.len() >= MAX_CHUNK;
            if boundary {
                let len = pending.len();
                let sha = hex::encode(Sha256::digest(&pending));
                plans.push(ChunkPlan {
                    sha256: sha,
                    offset: chunk_start,
                    len,
                });
                chunk_start += len as u64;
                pending.clear();
                gear = 0;
            }
        }
    }

    // Flush the trailing partial chunk.
    if !pending.is_empty() {
        let len = pending.len();
        let sha = hex::encode(Sha256::digest(&pending));
        plans.push(ChunkPlan {
            sha256: sha,
            offset: chunk_start,
            len,
        });
    }

    Ok(plans)
}

/// Read the `[offset, offset+len)` range of `src` and write it to `dest`
/// atomically (temp file + rename). Used by the commit phase to place a chunk
/// that isn't already on disk. Idempotent: a concurrent uploader that placed
/// the same chunk first just gets overwritten by an identical rename.
pub async fn place_chunk(src: &Path, offset: u64, len: usize, dest: &Path) -> std::io::Result<()> {
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let mut file = tokio::fs::File::open(src).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    let mut remaining = len;
    let tmp = dest.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
    {
        let mut out = tokio::fs::File::create(&tmp).await?;
        let mut buf = vec![0u8; 256 * 1024];
        while remaining > 0 {
            let want = remaining.min(buf.len());
            let n = file.read(&mut buf[..want]).await?;
            if n == 0 {
                break;
            }
            tokio::io::AsyncWriteExt::write_all(&mut out, &buf[..n]).await?;
            remaining -= n;
        }
        tokio::io::AsyncWriteExt::flush(&mut out).await?;
    }
    tokio::fs::rename(&tmp, dest).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same bytes ⇒ same chunk boundaries and shas, deterministically.
    #[tokio::test]
    async fn chunking_is_deterministic() {
        let dir = std::env::temp_dir().join(format!("hoard-chunk-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let p = dir.join("f.bin");
        // 10 MiB of pseudo-random but reproducible bytes.
        let mut data = vec![0u8; 10 << 20];
        let mut s: u64 = 12345;
        for b in data.iter_mut() {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (s >> 33) as u8;
        }
        tokio::fs::write(&p, &data).await.unwrap();

        let a = plan_chunks(&p).await.unwrap();
        let b = plan_chunks(&p).await.unwrap();
        assert_eq!(a.len(), b.len());
        assert!(a.len() > 1, "10 MiB should split into multiple chunks");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.sha256, y.sha256);
            assert_eq!(x.len, y.len);
            assert_eq!(x.offset, y.offset);
        }
        // Chunk sizes respect the bounds (except possibly the last).
        for (i, c) in a.iter().enumerate() {
            if i + 1 < a.len() {
                assert!(
                    c.len >= MIN_CHUNK && c.len <= MAX_CHUNK,
                    "chunk {i} len {}",
                    c.len
                );
            }
        }
        // Offsets + lengths tile the file exactly.
        let total: usize = a.iter().map(|c| c.len).sum();
        assert_eq!(total, data.len());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A small edit near the start re-chunks locally: most chunk shas survive.
    #[tokio::test]
    async fn local_edit_preserves_most_chunks() {
        let dir = std::env::temp_dir().join(format!("hoard-chunk-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let mut data = vec![0u8; 16 << 20];
        let mut s: u64 = 99;
        for b in data.iter_mut() {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *b = (s >> 33) as u8;
        }
        let p1 = dir.join("v1.bin");
        tokio::fs::write(&p1, &data).await.unwrap();

        // Flip a handful of bytes well into the file.
        for b in &mut data[(8 << 20)..((8 << 20) + 16)] {
            *b ^= 0xFF;
        }
        let p2 = dir.join("v2.bin");
        tokio::fs::write(&p2, &data).await.unwrap();

        let a = plan_chunks(&p1).await.unwrap();
        let b = plan_chunks(&p2).await.unwrap();
        let set_a: std::collections::HashSet<_> = a.iter().map(|c| c.sha256.clone()).collect();
        let shared = b.iter().filter(|c| set_a.contains(&c.sha256)).count();
        // The vast majority of chunks should be untouched by a 16-byte edit.
        assert!(
            shared * 2 > b.len(),
            "expected most chunks shared, got {shared}/{}",
            b.len()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// place_chunk extracts exactly the planned byte range.
    #[tokio::test]
    async fn place_chunk_roundtrip() {
        let dir = std::env::temp_dir().join(format!("hoard-chunk-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let p = dir.join("f.bin");
        let data: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        tokio::fs::write(&p, &data).await.unwrap();

        let dest = dir.join("chunk.bin");
        place_chunk(&p, 1000, 2048, &dest).await.unwrap();
        let got = tokio::fs::read(&dest).await.unwrap();
        assert_eq!(got, data[1000..3048]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
