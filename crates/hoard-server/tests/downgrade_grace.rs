//! The Pro→Free downgrade, end to end against a real Postgres.
//!
//! This is the rehearsal for the incident of ago-2026: a Pro account holding
//! 6.8 GB dropped to Free (2 GB) and the server shrank the limit the same
//! second, so the auto-purge deleted the user's version history with no notice
//! and every later upload bounced off a 402. The grace window meant to prevent
//! exactly that was dead code, `settle_storage_on_active` sized "how much room
//! do you have today" with the plan being moved *to*, so Pro→Free resolved to
//! 2 GB on both sides and never scheduled anything.
//!
//! It runs against a throwaway database instead of a paid subscription because
//! nothing here is Polar's decision: Polar only says "this subscription ended".
//! Everything that matters, grant, deadline, what the limit resolves to while
//! the plan column already says `free`, is [`quota::settle_storage_limit`] and
//! [`plans::resolved_storage_limit`], and both are reachable from a test.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set, like the S3 one. To run it:
//!
//! ```sh
//! docker run -d --name hoard-pg -p 55432:5432 \
//!   -e POSTGRES_PASSWORD=hoard -e POSTGRES_DB=hoard postgres:17
//! export HOARD_PG_TEST_URL=postgres://postgres:hoard@localhost:55432/hoard
//! cargo test -p hoard-server --features cloud --test downgrade_grace -- --nocapture
//! ```
//!
//! **Never point it at production.** It creates and deletes its own profile
//! rows, but the migrations run on whatever database it's given.

#![cfg(feature = "cloud")]

use hoard_server::cloud::plans::Plan;
use hoard_server::cloud::quota::{self, SettleOutcome};
use sqlx::PgPool;
use uuid::Uuid;

const GB: i64 = 1024 * 1024 * 1024;
const GRACE_DAYS: i64 = 30;

/// Connect + migrate, or `None` when the env var isn't set (CI's no-op path).
async fn pool() -> Option<PgPool> {
    let url = std::env::var("HOARD_PG_TEST_URL").ok()?;
    let pool = hoard_server::cloud::db::connect(&url, 5)
        .await
        .expect("connect to the test database");
    // Serialize the bootstrap across the whole binary. `cargo test` runs these
    // in parallel and they all set up the same Supabase stand-ins, which had
    // them racing on `CREATE OR REPLACE FUNCTION` ("tuple concurrently
    // updated"). An advisory lock is the fix rather than telling everyone to
    // remember `--test-threads=1`, which only works until someone forgets.
    sqlx::query("SELECT pg_advisory_lock(8_233_119_402)")
        .execute(&pool)
        .await
        .expect("bootstrap lock");
    // The migrations assume Supabase: an `auth.users` table to hang the
    // `profiles` FK off (0013), an `auth.uid()` for the RLS policies, and the
    // `anon` / `authenticated` roles the admin-metrics grants name (0030). A
    // bare Postgres has none of them, so stand up the shapes they reference.
    // Nothing here authenticates anything, RLS is never the path the server
    // takes (it connects as the owner); the objects just have to be creatable.
    for role in ["anon", "authenticated", "service_role"] {
        // No IF NOT EXISTS for roles before PG 16's syntax, and re-running the
        // suite must stay idempotent, so swallow the duplicate.
        let _ = sqlx::query(&format!("CREATE ROLE {role} NOLOGIN"))
            .execute(&pool)
            .await;
    }
    sqlx::query("CREATE SCHEMA IF NOT EXISTS auth")
        .execute(&pool)
        .await
        .expect("auth schema");
    sqlx::query("CREATE TABLE IF NOT EXISTS auth.users (id UUID PRIMARY KEY)")
        .execute(&pool)
        .await
        .expect("auth.users");
    sqlx::query(
        "CREATE OR REPLACE FUNCTION auth.uid() RETURNS UUID LANGUAGE sql STABLE AS $$ SELECT NULL::uuid $$",
    )
    .execute(&pool)
    .await
    .expect("auth.uid()");
    hoard_server::cloud::db::run_migrations(&pool)
        .await
        .expect("migrations");
    sqlx::query("SELECT pg_advisory_unlock(8_233_119_402)")
        .execute(&pool)
        .await
        .expect("bootstrap unlock");
    Some(pool)
}

/// A Pro profile storing `used` bytes, on the base tier (no bought override).
async fn seed_pro(pool: &PgPool, used: i64) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth.users (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(id)
        .execute(pool)
        .await
        .expect("auth user");
    sqlx::query(
        "INSERT INTO profiles (user_id, email, plan, storage_bytes) VALUES ($1, $2, 'pro', $3)",
    )
    .bind(id)
    .bind(format!("{id}@test.invalid"))
    .bind(used)
    .execute(pool)
    .await
    .expect("profile");
    id
}

/// What the webhook does after settling: flip the plan column.
async fn set_plan(pool: &PgPool, id: Uuid, plan: &str) {
    sqlx::query("UPDATE profiles SET plan = $1 WHERE user_id = $2")
        .bind(plan)
        .bind(id)
        .execute(pool)
        .await
        .expect("plan flip");
}

async fn enforced_limit(pool: &PgPool, id: Uuid) -> u64 {
    let (limits, _info) = quota::load(pool, id)
        .await
        .expect("quota load")
        .expect("profile exists");
    limits.storage_bytes
}

async fn cleanup(pool: &PgPool, id: Uuid) {
    let _ = sqlx::query("DELETE FROM profiles WHERE user_id = $1")
        .bind(id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM auth.users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await;
}

/// The regression itself: a Pro account over Free's limit keeps its old room
/// until the deadline, *including after the plan column says `free`*, and only
/// then collapses.
#[tokio::test]
async fn pro_to_free_over_footprint_gets_the_window() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    // The real account: 6.8 GB stored, dropping to a 2 GB plan.
    let id = seed_pro(&pool, 6_800_000_000).await;

    let outcome = quota::settle_storage_limit(&pool, id, Plan::Free, None, GRACE_DAYS)
        .await
        .expect("settle");
    assert_eq!(
        outcome,
        SettleOutcome::Scheduled {
            grant_bytes: 100 * GB,
            target_bytes: 2 * GB,
        },
        "a downgrade below the footprint schedules, it doesn't apply"
    );

    // The webhook flips the plan right after. This is the exact moment the old
    // code lost: `plan` says free, so a limit derived from the plan alone is
    // 2 GB and the purge starts eating history.
    set_plan(&pool, id, "free").await;
    assert_eq!(
        enforced_limit(&pool, id).await,
        100 * GB as u64,
        "inside the window the old limit still rules, plan column notwithstanding"
    );

    // Webhook retries / the `/v1/me` expiry sweep must not push the deadline.
    let before: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT storage_limit_change_at FROM profiles WHERE user_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("deadline");
    let again = quota::settle_storage_limit(&pool, id, Plan::Free, None, GRACE_DAYS)
        .await
        .expect("settle again");
    assert_eq!(
        again,
        SettleOutcome::AlreadyScheduled {
            target_bytes: 2 * GB
        },
    );
    let after: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT storage_limit_change_at FROM profiles WHERE user_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("deadline");
    assert_eq!(before, after, "a second event can't extend the window");

    // Wind the clock past the deadline: now, and only now, it shrinks.
    sqlx::query("UPDATE profiles SET storage_limit_change_at = now() - interval '1 minute' WHERE user_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("rewind");
    quota::apply_due_downgrade(&pool, id)
        .await
        .expect("promote");
    assert_eq!(
        enforced_limit(&pool, id).await,
        2 * GB as u64,
        "past the deadline the Free limit applies"
    );
    let leftovers: (Option<i64>, Option<time::OffsetDateTime>) = sqlx::query_as(
        "SELECT pending_storage_limit_bytes, storage_limit_change_at FROM profiles WHERE user_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("columns");
    assert_eq!(leftovers, (None, None), "the window clears itself");

    cleanup(&pool, id).await;
}

/// A downgrade the account already fits in is not a downgrade to warn about:
/// it applies at once, with no window and nothing to count down to.
#[tokio::test]
async fn pro_to_free_within_the_limit_applies_immediately() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let id = seed_pro(&pool, 500 * 1024 * 1024).await; // 500 MB, fits in Free

    let outcome = quota::settle_storage_limit(&pool, id, Plan::Free, None, GRACE_DAYS)
        .await
        .expect("settle");
    assert_eq!(
        outcome,
        SettleOutcome::Applied {
            limit_bytes: 2 * GB
        }
    );
    set_plan(&pool, id, "free").await;
    assert_eq!(enforced_limit(&pool, id).await, 2 * GB as u64);

    cleanup(&pool, id).await;
}

/// Coming back to Pro during the window cancels it outright, no lingering
/// deadline waiting to shrink a paying account.
#[tokio::test]
async fn resubscribing_cancels_a_pending_downgrade() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let id = seed_pro(&pool, 6_800_000_000).await;
    quota::settle_storage_limit(&pool, id, Plan::Free, None, GRACE_DAYS)
        .await
        .expect("settle down");
    set_plan(&pool, id, "free").await;

    // Polar says active again. Settle *then* flip, the order the webhook uses.
    let outcome = quota::settle_storage_limit(&pool, id, Plan::Pro, None, GRACE_DAYS)
        .await
        .expect("settle up");
    assert_eq!(
        outcome,
        SettleOutcome::Applied {
            limit_bytes: 100 * GB
        }
    );
    set_plan(&pool, id, "pro").await;
    assert_eq!(enforced_limit(&pool, id).await, 100 * GB as u64);
    let pending: (Option<i64>, Option<time::OffsetDateTime>) = sqlx::query_as(
        "SELECT pending_storage_limit_bytes, storage_limit_change_at FROM profiles WHERE user_id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("columns");
    assert_eq!(pending, (None, None), "no downgrade left pending");

    cleanup(&pool, id).await;
}

/// The "same folder tracked twice" shape, which is what made 1.25 GB invisible
/// on a real account: two live saves referencing the same blob. Neither can
/// claim those bytes as exclusive, so `freeable_bytes` reports 0 for both and
/// archiving either one alone frees nothing, `shared_groups` is what makes
/// them visible.
///
/// It also pins the decode. `array_agg(f.save_id)` yields `text[]` **because
/// `saves.id` is TEXT**; were it UUID the query would still compile and then
/// 500 in production on every open of the "free up space" dialog.
#[tokio::test]
async fn shared_blobs_between_two_saves_are_reported_as_a_group() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let id = seed_pro(&pool, 0).await;
    let twin_a = format!("save-a-{id}");
    let twin_b = format!("save-b-{id}");
    let lonely = format!("save-c-{id}");
    let shared_sha = format!("sha-shared-{id}");
    let own_sha = format!("sha-own-{id}");

    for (save_id, slug) in [
        (&twin_a, "surviving-mars-relaunched"),
        (&twin_b, "mars"),
        (&lonely, "factorio"),
    ] {
        sqlx::query(
            "INSERT INTO saves (id, user_id, game_slug, label, latest_version_num)
             VALUES ($1, $2, $3, 'main', 1)",
        )
        .bind(save_id)
        .bind(id)
        .bind(slug)
        .execute(&pool)
        .await
        .expect("save");
        sqlx::query(
            "INSERT INTO save_versions (save_id, version_num, size_bytes, sha256, r2_key, content_addressed)
             VALUES ($1, 1, 10, 'v', '', TRUE)",
        )
        .bind(save_id)
        .execute(&pool)
        .await
        .expect("version");
    }

    // The twins both point at the shared blob; the third save has its own.
    for (save_id, sha) in [
        (&twin_a, &shared_sha),
        (&twin_b, &shared_sha),
        (&lonely, &own_sha),
    ] {
        sqlx::query(
            "INSERT INTO save_version_files (save_id, version_num, relative_path, sha256, size_bytes)
             VALUES ($1, 1, 'save.dat', $2, 1000)",
        )
        .bind(save_id)
        .bind(sha)
        .execute(&pool)
        .await
        .expect("file row");
    }
    sqlx::query(
        "INSERT INTO cloud_blobs (user_id, sha256, size_bytes, r2_key, refcount)
         VALUES ($1, $2, 1000, 'k1', 2), ($1, $3, 500, 'k2', 1)",
    )
    .bind(id)
    .bind(&shared_sha)
    .bind(&own_sha)
    .execute(&pool)
    .await
    .expect("blobs");

    let groups = hoard_server::cloud::archive::shared_groups(&pool, id)
        .await
        .expect("shared groups");
    assert_eq!(groups.len(), 1, "only the twins share: {groups:?}");
    assert_eq!(groups[0].bytes, 1000);
    let mut ids = groups[0].save_ids.clone();
    ids.sort();
    let mut want = vec![twin_a.clone(), twin_b.clone()];
    want.sort();
    assert_eq!(ids, want);

    // Archiving one twin retires it from the group: its references are already
    // released, so it can no longer hold the other's bytes hostage.
    sqlx::query("UPDATE saves SET archived_at = now() WHERE id = $1")
        .bind(&twin_a)
        .execute(&pool)
        .await
        .expect("archive one");
    let groups = hoard_server::cloud::archive::shared_groups(&pool, id)
        .await
        .expect("shared groups after archive");
    assert!(
        groups.is_empty(),
        "one live save left holding the blob is not a shared group: {groups:?}"
    );

    for save_id in [&twin_a, &twin_b, &lonely] {
        let _ = sqlx::query("DELETE FROM saves WHERE id = $1")
            .bind(save_id)
            .execute(&pool)
            .await;
    }
    let _ = sqlx::query("DELETE FROM cloud_blobs WHERE user_id = $1")
        .bind(id)
        .execute(&pool)
        .await;
    cleanup(&pool, id).await;
}

/// Storage comes back on a downgrade; devices don't. An account that was ever
/// Pro keeps its machines for life, dropping six paired PCs to three the day a
/// subscription lapses is a working account turning into a broken-looking one.
#[tokio::test]
async fn an_ex_pro_account_keeps_its_devices_after_dropping_to_free() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let id = seed_pro(&pool, 500 * 1024 * 1024).await;
    // The webhook stamps this the first time the account goes Pro.
    sqlx::query("UPDATE profiles SET first_pro_at = now() - interval '60 days' WHERE user_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("stamp");

    quota::settle_storage_limit(&pool, id, Plan::Free, None, GRACE_DAYS)
        .await
        .expect("settle");
    set_plan(&pool, id, "free").await;

    let (limits, _info) = quota::load(&pool, id)
        .await
        .expect("load")
        .expect("profile");
    assert_eq!(
        limits.storage_bytes,
        2 * GB as u64,
        "the storage does go back"
    );
    assert_eq!(
        limits.devices,
        Plan::Pro.limits().devices,
        "the devices do not"
    );

    // A neighbour who never paid gets the plain Free cap, so the grandfathering
    // is the marker's doing and not a blanket grant.
    let plain = seed_pro(&pool, 0).await;
    set_plan(&pool, plain, "free").await;
    let (plain_limits, _) = quota::load(&pool, plain)
        .await
        .expect("load")
        .expect("profile");
    assert_eq!(plain_limits.devices, 3);

    cleanup(&pool, id).await;
    cleanup(&pool, plain).await;
}
