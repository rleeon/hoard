//! Backfill of the interned manifest (phase 3, step 3).
//!
//! Every commit already writes both shapes, so this only has to catch up on
//! what was committed before that landed. It walks save by save rather than
//! doing one enormous statement, for two reasons: a single `INSERT ... SELECT`
//! over 629k rows holds one transaction open for its whole duration, and the
//! dead tuples it leaves behind cannot be reused until it ends. Per save, the
//! bloat stays bounded and a `VACUUM` between batches can return it.
//!
//! It is idempotent. Both inserts are `ON CONFLICT DO NOTHING` against the
//! natural keys, so a re-run after a crash resumes rather than duplicating, and
//! a save that races with a live commit ends up with the same rows either way.
//!
//! Nothing reads the interned tables yet. This can run as many times as it
//! takes, in the middle of the day, without a user noticing.

use anyhow::{Context, Result};
use sqlx::{PgPool, Row};

/// What one pass did.
#[derive(Debug, Default)]
pub struct Report {
    pub saves: usize,
    pub entries: u64,
    pub references: u64,
    /// Saves whose two representations disagreed afterwards. Not an error the
    /// pass can fix, and the reason the cutover has a verification step of its
    /// own: a non-empty list here means something is writing one shape and not
    /// the other, and the reads must not switch over until it is empty.
    pub mismatched: Vec<String>,
}

pub struct Options {
    /// Stop after this many saves. `None` walks all of them.
    pub limit: Option<i64>,
    /// Report what would be written without writing it.
    pub dry_run: bool,
}

/// Fill the catalogue and the references for one save, then report how the two
/// representations compare. Returns `(entries, references)` written.
async fn one_save(pool: &PgPool, save_id: &str) -> Result<(u64, u64)> {
    // The catalogue first: every distinct (path, sha, size) this save has ever
    // held. `DISTINCT` rather than `ON CONFLICT` alone because the source can
    // hold the same triple hundreds of times and there is no point offering it
    // hundreds of times.
    let entries = sqlx::query(
        "INSERT INTO file_entries (save_id, relative_path, sha256, size_bytes)
         SELECT DISTINCT save_id, relative_path, sha256, size_bytes
           FROM save_version_files
          WHERE save_id = $1
         ON CONFLICT (save_id, relative_path, sha256) DO NOTHING",
    )
    .bind(save_id)
    .execute(pool)
    .await
    .with_context(|| format!("catalogue for {save_id}"))?
    .rows_affected();

    // Then the references. The join to `save_versions` is what turns
    // (save_id, version_num) into the surrogate id the references carry; the
    // join to `file_entries` resolves the content to its catalogue row, and by
    // now every row it needs exists.
    let references = sqlx::query(
        "INSERT INTO version_files (version_id, entry_id, modified_at)
         SELECT v.id, e.id, f.modified_at
           FROM save_version_files f
           JOIN save_versions v
             ON v.save_id = f.save_id AND v.version_num = f.version_num
           JOIN file_entries e
             ON e.save_id = f.save_id
            AND e.relative_path = f.relative_path
            AND e.sha256 = f.sha256
          WHERE f.save_id = $1
         ON CONFLICT DO NOTHING",
    )
    .bind(save_id)
    .execute(pool)
    .await
    .with_context(|| format!("references for {save_id}"))?
    .rows_affected();

    Ok((entries, references))
}

/// True when the interned tables reproduce the old table exactly for this save,
/// in both directions: nothing missing, nothing invented.
async fn agrees(pool: &PgPool, save_id: &str) -> Result<bool> {
    let row = sqlx::query(
        "SELECT
           (SELECT count(*) FROM save_version_files f
             WHERE f.save_id = $1
               AND NOT EXISTS (
                   SELECT 1 FROM version_files vf
                     JOIN save_versions v ON v.id = vf.version_id
                     JOIN file_entries e ON e.id = vf.entry_id
                    WHERE v.save_id = f.save_id
                      AND v.version_num = f.version_num
                      AND e.relative_path = f.relative_path
                      AND e.sha256 = f.sha256
                      AND e.size_bytes = f.size_bytes
                      AND vf.modified_at IS NOT DISTINCT FROM f.modified_at)) AS missing,
           (SELECT count(*) FROM version_files vf
              JOIN save_versions v ON v.id = vf.version_id
              JOIN file_entries e ON e.id = vf.entry_id
             WHERE v.save_id = $1
               AND NOT EXISTS (
                   SELECT 1 FROM save_version_files f
                    WHERE f.save_id = v.save_id
                      AND f.version_num = v.version_num
                      AND f.relative_path = e.relative_path
                      AND f.sha256 = e.sha256)) AS invented",
    )
    .bind(save_id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("comparison for {save_id}"))?;

    let missing: i64 = row.get("missing");
    let invented: i64 = row.get("invented");
    Ok(missing == 0 && invented == 0)
}

/// Walk every save that still has rows the interned tables have not caught up
/// on. Saves already complete are skipped, which is what makes a re-run cheap.
pub async fn run(cfg: &crate::config::Config, opts: Options) -> Result<Report> {
    let pool = super::db::connect(&cfg.database.url, cfg.database.max_connections)
        .await
        .context("backfill: connecting to Postgres")?;

    let mut sql = String::from(
        "SELECT DISTINCT f.save_id
           FROM save_version_files f
          WHERE NOT EXISTS (
                SELECT 1 FROM version_files vf
                  JOIN save_versions v ON v.id = vf.version_id
                  JOIN file_entries e ON e.id = vf.entry_id
                 WHERE v.save_id = f.save_id
                   AND v.version_num = f.version_num
                   AND e.relative_path = f.relative_path
                   AND e.sha256 = f.sha256)
          ORDER BY f.save_id",
    );
    if let Some(n) = opts.limit {
        sql.push_str(&format!(" LIMIT {n}"));
    }
    let pending: Vec<(String,)> = sqlx::query_as(&sql)
        .fetch_all(&pool)
        .await
        .context("backfill: listing saves")?;

    let mut report = Report {
        saves: pending.len(),
        ..Default::default()
    };
    if opts.dry_run {
        return Ok(report);
    }

    for (save_id,) in &pending {
        let (entries, references) = one_save(&pool, save_id).await?;
        report.entries += entries;
        report.references += references;
        if !agrees(&pool, save_id).await? {
            report.mismatched.push(save_id.clone());
        }
        tracing::info!(save_id = %save_id, entries, references, "manifest backfill: save done");
    }

    Ok(report)
}

pub fn print_report(report: &Report, dry_run: bool) {
    if dry_run {
        println!("{} saves would be backfilled", report.saves);
        return;
    }
    println!(
        "{} saves: {} catalogue entries, {} references",
        report.saves, report.entries, report.references
    );
    if report.mismatched.is_empty() {
        println!("every save checked reproduces the old table exactly");
    } else {
        println!(
            "{} saves DISAGREE and the reads must not be switched over: {:?}",
            report.mismatched.len(),
            report.mismatched
        );
    }
}
