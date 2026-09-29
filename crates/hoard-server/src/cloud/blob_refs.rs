//! Taking and handing back blob references, inside the caller's transaction.
//!
//! The rule is the one `cas_commit` applies when a version lands: +1 on each
//! distinct blob the version names. Everything that removes versions has to
//! undo exactly that, and in the same transaction as the removal. Before, the
//! release ran after the save was already gone, one blob at a time, tied to the
//! request: a client that gave up at 60 s left the rest charged forever
//! (September 2026, ten accounts, up to 976 MB each). It also counted pending
//! versions, which never took a reference, so deleting a save with an upload
//! in flight took references that belonged to other saves sharing the file.
//!
//! Nothing here touches the bucket. A blob that reaches 0 gets `purge_after`
//! and the GC in `archive::purge_due_blobs` deletes the object, then the row.

use sqlx::PgConnection;
use uuid::Uuid;

/// Distinct blobs referenced by the committed versions of `save_id` (or of one
/// version), with how many versions reference each, sorted by sha.
pub async fn committed_refs(
    conn: &mut PgConnection,
    save_id: &str,
    version: Option<i64>,
) -> Result<Vec<(Vec<u8>, i64)>, sqlx::Error> {
    sqlx::query_as(
        "SELECT fe.sha256, count(DISTINCT v.id)
           FROM save_versions v
           JOIN version_files vf ON vf.version_id = v.id
           JOIN file_entries fe ON fe.id = vf.entry_id
          WHERE v.save_id = $1 AND v.sha256 <> '' AND v.content_addressed
            AND ($2::bigint IS NULL OR v.version_num = $2)
          GROUP BY fe.sha256
          ORDER BY fe.sha256",
    )
    .bind(save_id)
    .bind(version)
    .fetch_all(conn)
    .await
}

/// Lock the rows in sha order before changing them. `cas_commit` upserts in
/// the same order, so two of these in flight for one account queue instead of
/// deadlocking.
async fn lock(conn: &mut PgConnection, user_id: Uuid, shas: &[Vec<u8>]) -> Result<(), sqlx::Error> {
    sqlx::query(
        "SELECT 1 FROM cloud_blobs
          WHERE user_id = $1 AND sha256 = ANY($2)
          ORDER BY sha256
            FOR UPDATE",
    )
    .bind(user_id)
    .bind(shas)
    .execute(conn)
    .await?;
    Ok(())
}

/// Drop the references. A blob that reaches 0 is due for collection
/// `grace_secs` from now; the storage trigger credits its size on that same
/// transition, so the quota drops when the caller commits.
pub async fn release(
    conn: &mut PgConnection,
    user_id: Uuid,
    refs: &[(Vec<u8>, i64)],
    grace_secs: i64,
) -> Result<(), sqlx::Error> {
    if refs.is_empty() {
        return Ok(());
    }
    let (shas, counts): (Vec<Vec<u8>>, Vec<i64>) = refs.iter().cloned().unzip();
    lock(&mut *conn, user_id, &shas).await?;
    sqlx::query(
        "UPDATE cloud_blobs b
            SET refcount = GREATEST(0, b.refcount - t.n),
                purge_after = CASE WHEN b.refcount - t.n <= 0
                                   THEN now() + make_interval(secs => $4)
                                   ELSE b.purge_after END
           FROM unnest($2::bytea[], $3::bigint[]) AS t(sha, n)
          WHERE b.user_id = $1 AND b.sha256 = t.sha",
    )
    .bind(user_id)
    .bind(&shas)
    .bind(&counts)
    .bind(grace_secs as f64)
    .execute(conn)
    .await?;
    Ok(())
}

/// Take the references back (reactivating an archived save). Clears
/// `purge_after`, and the trigger charges the storage again on 0 to >0.
pub async fn retake(
    conn: &mut PgConnection,
    user_id: Uuid,
    refs: &[(Vec<u8>, i64)],
) -> Result<(), sqlx::Error> {
    if refs.is_empty() {
        return Ok(());
    }
    let (shas, counts): (Vec<Vec<u8>>, Vec<i64>) = refs.iter().cloned().unzip();
    lock(&mut *conn, user_id, &shas).await?;
    sqlx::query(
        "UPDATE cloud_blobs b
            SET refcount = b.refcount + t.n, purge_after = NULL
           FROM unnest($2::bytea[], $3::bigint[]) AS t(sha, n)
          WHERE b.user_id = $1 AND b.sha256 = t.sha",
    )
    .bind(user_id)
    .bind(&shas)
    .bind(&counts)
    .execute(conn)
    .await?;
    Ok(())
}
