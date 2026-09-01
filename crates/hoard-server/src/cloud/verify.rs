//! `hoard-server verify-blobs`: the forensic pass over the cloud's blobs.
//!
//! A blob is content-addressed: its name *is* the sha256 of its content. Nothing
//! on the normal path ever checks that again. The commit does a HEAD (which only
//! tells you the size) and the compression sweep does verify what it compresses
//! itself, but an object that was uploaded wrong and never compressed is looked at
//! by nobody: it gets discovered on restore, which is the worst possible moment.
//!
//! And some were uploaded wrong. Until aug-2026 the client hashed the file and
//! uploaded it in two separate reads; if the game rotated the save in between
//! (`save` to `save.bak` with a new one in its place, the ordinary autosave
//! pattern) the object ended up holding bytes that are not what its name promises.
//! The client can no longer produce them (it hashes the PUT's own stream and
//! aborts before confirming the version), but the ones from back then are still
//! there.
//!
//! This finds them: it reads each object whole, decompresses if needed, hashes and
//! compares. It writes the verdict into `cloud_blobs.integrity` so it does not
//! re-read what is already healthy on the next pass, and it names the affected
//! saves (user, game, version and file) because what the operator needs to know is
//! not "blob abc123 is lying" but "this person's restore is going to come back
//! wrong".
//!
//! It only reads and records. It deletes nothing: a broken blob is still the only
//! copy of that version there is, and throwing it away would trade "restores
//! wrong" for "does not restore". What to do with them is a product decision, not
//! a sweep's.

use anyhow::{Context, Result};
use async_compression::tokio::bufread::ZstdDecoder;
use futures::{stream, StreamExt};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::io::{AsyncBufRead, AsyncReadExt};

use crate::config::Config;

/// What the pass is asked for.
#[derive(Debug, Clone)]
pub struct Options {
    /// How many blobs to look at at most. `None` means all that are left.
    pub limit: Option<i64>,
    /// Look again at the ones that already have a verdict.
    pub recheck: bool,
    /// Lecturas simultáneas.
    pub concurrency: usize,
    /// Do not write verdicts to the database, only report.
    pub dry_run: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            limit: None,
            recheck: false,
            // Every read is a whole download from R2: a few at a time saturate
            // the link without leaving the machine no air to serve with.
            concurrency: 4,
            dry_run: false,
        }
    }
}

/// One object's verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The bytes hash to its sha256.
    Ok,
    /// The object is there, but its content is not what its name promises.
    Mismatch,
    /// Not in the bucket.
    Missing,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Ok => "ok",
            Verdict::Mismatch => "mismatch",
            Verdict::Missing => "missing",
        }
    }
}

/// A blob that failed, with what is needed to warn whoever cares.
#[derive(Debug, Clone)]
pub struct Damage {
    pub user_id: uuid::Uuid,
    pub sha256: String,
    pub verdict: Verdict,
    /// What was actually read (empty when the object was missing).
    pub actual_sha256: String,
    /// `(game_slug, version_num, relative_path)` of every place that uses it.
    pub used_by: Vec<(String, i64, String)>,
}

#[derive(Debug, Default, Clone)]
pub struct Report {
    pub checked: usize,
    pub ok: usize,
    pub damaged: Vec<Damage>,
}

/// Hash of the object's *logical* content: it decompresses before hashing when
/// the blob is stored as zstd, because the sha256 its name promises is the one of
/// the original bytes, not of their packaging.
///
/// Streaming, with a fixed buffer: there are blobs of hundreds of MB and this
/// runs on a 512 MB machine.
pub async fn digest_of<R: AsyncBufRead + Unpin>(reader: R, zstd: bool) -> Result<(String, u64)> {
    let mut hasher = Sha256::new();
    let mut len = 0u64;
    let mut buf = vec![0u8; 256 * 1024];
    if zstd {
        let mut dec = ZstdDecoder::new(reader);
        loop {
            let n = dec.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            len += n as u64;
        }
    } else {
        let mut r = reader;
        loop {
            let n = r.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            len += n as u64;
        }
    }
    Ok((hex::encode(hasher.finalize()), len))
}

/// Compares what was read against what was promised. Pure: the verdict is the
/// part worth a test, and it needs neither bucket nor database.
pub fn judge(declared_sha: &str, declared_len: i64, actual: Option<(&str, u64)>) -> Verdict {
    match actual {
        None => Verdict::Missing,
        Some((sha, len)) => {
            // Size counts too: `size_bytes` is what the user is charged and what
            // the restore expects. An object that hashes right but measures
            // something else is just as useless.
            if sha == declared_sha && len == declared_len.max(0) as u64 {
                Verdict::Ok
            } else {
                Verdict::Mismatch
            }
        }
    }
}

/// Runs the pass. Returns the report; printing it is the caller's business.
pub async fn run(cfg: &Config, opts: Options) -> Result<Report> {
    let cloud_cfg = cfg
        .cloud
        .as_ref()
        .context("verify-blobs: this server has no [cloud] section")?;
    let pool = super::db::connect(&cfg.database.url, cfg.database.max_connections)
        .await
        .context("verify-blobs: connecting to Postgres")?;
    let r2 = super::r2::R2Store::from_config(&cloud_cfg.r2)
        .await
        .context("verify-blobs: building the R2 client")?;

    let mut sql = String::from(
        "SELECT user_id, sha256, size_bytes, r2_key, encoding, stored_bytes
           FROM cloud_blobs",
    );
    if !opts.recheck {
        sql.push_str(" WHERE verified_at IS NULL");
    }
    sql.push_str(" ORDER BY created_at");
    if let Some(n) = opts.limit {
        sql.push_str(&format!(" LIMIT {n}"));
    }
    let rows = sqlx::query(&sql).fetch_all(&pool).await?;

    let checks = rows.iter().map(|row| {
        let r2 = &r2;
        async move {
            let user_id: uuid::Uuid = row.get("user_id");
            let sha: String = row.get("sha256");
            let size: i64 = row.get("size_bytes");
            let key: String = row.get("r2_key");
            let encoding: Option<String> = row.get("encoding");
            let stored: Option<i64> = row.get("stored_bytes");
            // Decompression is only needed when the sweep *finished* writing the
            // compressed version; `encoding='zstd'` with a NULL `stored_bytes` is
            // a claim in progress and the object is still raw.
            let zstd = encoding.as_deref() == Some("zstd") && stored.is_some();

            let actual = match r2.get_reader(&key).await {
                Ok(reader) => Some(digest_of(reader, zstd).await?),
                // Any read failure is treated as "not there": if the object
                // cannot be downloaded, a restore will not manage either.
                Err(e) => {
                    tracing::debug!(error = %e, key = %key, "verify-blobs: unreadable object");
                    None
                }
            };
            let verdict = judge(&sha, size, actual.as_ref().map(|(s, l)| (s.as_str(), *l)));
            Ok::<_, anyhow::Error>((
                user_id,
                sha,
                verdict,
                actual.map(|(s, _)| s).unwrap_or_default(),
            ))
        }
    });

    let results: Vec<(uuid::Uuid, String, Verdict, String)> = stream::iter(checks)
        .buffer_unordered(opts.concurrency)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>>>()?;

    let mut report = Report {
        checked: results.len(),
        ..Default::default()
    };
    for (user_id, sha, verdict, actual) in results {
        if !opts.dry_run {
            sqlx::query(
                "UPDATE cloud_blobs SET verified_at = now(), integrity = $3
                  WHERE user_id = $1 AND sha256 = $2",
            )
            .bind(user_id)
            .bind(&sha)
            .bind(verdict.as_str())
            .execute(&pool)
            .await?;
        }
        if verdict == Verdict::Ok {
            report.ok += 1;
            continue;
        }
        // Who suffers it. A blob can be referenced by several versions (dedup),
        // and the operator needs the whole list to warn people.
        let used_by: Vec<(String, i64, String)> = sqlx::query(
            "SELECT s.game_slug, f.version_num, f.relative_path
               FROM save_version_files f
               JOIN saves s ON s.id = f.save_id
              WHERE f.sha256 = $1 AND s.user_id = $2
              ORDER BY s.game_slug, f.version_num",
        )
        .bind(&sha)
        .bind(user_id)
        .fetch_all(&pool)
        .await?
        .into_iter()
        .map(|r| {
            (
                r.get("game_slug"),
                r.get("version_num"),
                r.get("relative_path"),
            )
        })
        .collect();

        tracing::warn!(
            user_id = %user_id,
            sha = %sha,
            verdict = verdict.as_str(),
            actual = %actual,
            references = used_by.len(),
            "verify-blobs: damaged blob"
        );
        report.damaged.push(Damage {
            user_id,
            sha256: sha,
            verdict,
            actual_sha256: actual,
            used_by,
        });
    }
    Ok(report)
}

/// A readable report for the `fly ssh console -C ...` console.
pub fn print_report(report: &Report, dry_run: bool) {
    println!(
        "verify-blobs: {} checked, {} ok, {} damaged{}",
        report.checked,
        report.ok,
        report.damaged.len(),
        if dry_run {
            " (dry run, nothing written)"
        } else {
            ""
        }
    );
    for d in &report.damaged {
        println!(
            "  {} {} — got {}",
            d.verdict.as_str(),
            d.sha256,
            if d.actual_sha256.is_empty() {
                "nothing"
            } else {
                &d.actual_sha256
            }
        );
        println!("    user {}", d.user_id);
        for (game, version, path) in &d.used_by {
            println!("    used by {game} v{version} — {path}");
        }
        if d.used_by.is_empty() {
            println!("    unreferenced (orphan; safe to delete)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_compression::tokio::bufread::ZstdEncoder;

    async fn zstd_of(bytes: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = ZstdEncoder::new(bytes);
        enc.read_to_end(&mut out).await.unwrap();
        out
    }

    /// The hash is of the logical content: a compressed blob has to give the same
    /// sha as the raw one, or the pass would declare every healthy compressed
    /// blob broken.
    #[tokio::test]
    async fn the_digest_is_of_the_content_not_of_its_packaging() {
        let content = b"partida de ayer, con sus cosas dentro".repeat(500);
        let want = hex::encode(Sha256::digest(&content));

        let (raw_sha, raw_len) = digest_of(&content[..], false).await.unwrap();
        assert_eq!(raw_sha, want);
        assert_eq!(raw_len, content.len() as u64);

        let packed = zstd_of(&content).await;
        assert!(
            packed.len() < content.len(),
            "el zstd tiene que comprimir algo"
        );
        let (zstd_sha, zstd_len) = digest_of(&packed[..], true).await.unwrap();
        assert_eq!(zstd_sha, want, "descomprimido, es el mismo contenido");
        assert_eq!(zstd_len, content.len() as u64);
    }

    /// The verdict, which is what decides whether a user gets warned.
    #[test]
    fn the_verdict_needs_both_the_hash_and_the_size() {
        let sha = "a".repeat(64);
        assert_eq!(judge(&sha, 10, Some((&sha, 10))), Verdict::Ok);
        assert_eq!(judge(&sha, 10, None), Verdict::Missing);
        // The aug-2026 rotation: different content, same size.
        assert_eq!(
            judge(&sha, 10, Some((&"b".repeat(64), 10))),
            Verdict::Mismatch
        );
        // And an object that hashes right but measures something else is no good
        // either.
        assert_eq!(judge(&sha, 10, Some((&sha, 9))), Verdict::Mismatch);
    }
}
