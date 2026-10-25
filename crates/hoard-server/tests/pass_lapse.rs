//! A pass, end to end against a real Postgres: the grace month after its year,
//! a renewal inside that month continuing the old year, and a lapsed row that
//! must not drop an account still holding another paid one.
//!
//! Skipped unless `HOARD_PG_TEST_URL` is set; see `downgrade_grace` for the
//! container. **Never point it at production.**

#![cfg(feature = "cloud")]

use hoard_server::cloud::lapse::{self, Revoked};
use hoard_server::cloud::polar;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

const STORAGE_GRACE_DAYS: i64 = 30;

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
    //
    // Held on one connection, never on the pool. `pg_advisory_lock` is
    // session-scoped, so through a pool the unlock can land on a different
    // connection, quietly return false, and leave the lock held for the life of
    // that session; every later test binary then blocks on it forever. That
    // hung the suite once.
    let mut guard = pool.acquire().await.expect("bootstrap connection");
    sqlx::query("SELECT pg_advisory_lock(8_233_119_402)")
        .execute(&mut *guard)
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
        .execute(&mut *guard)
        .await
        .expect("bootstrap unlock");
    drop(guard);
    Some(pool)
}

async fn seed_pro(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO auth.users (id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(id)
        .execute(pool)
        .await
        .expect("auth user");
    sqlx::query("INSERT INTO profiles (user_id, email, plan) VALUES ($1, $2, 'pro')")
        .bind(id)
        .bind(format!("{id}@test.invalid"))
        .execute(pool)
        .await
        .expect("profile");
    id
}

async fn add_row(pool: &PgPool, user: Uuid, id: &str, interval: &str, renews_at: OffsetDateTime) {
    sqlx::query(
        r#"INSERT INTO subscriptions
               (user_id, ls_customer_id, ls_subscription_id, ls_product_id,
                plan, "interval", status, renews_at)
           VALUES ($1, 'cus', $2, 'prod', 'pro', $3, 'active', $4)"#,
    )
    .bind(user)
    .bind(id)
    .bind(interval)
    .bind(renews_at)
    .execute(pool)
    .await
    .expect("subscription row");
}

async fn plan(pool: &PgPool, user: Uuid) -> String {
    sqlx::query_scalar("SELECT plan FROM profiles WHERE user_id = $1")
        .bind(user)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn mark_ever_pro(pool: &PgPool, user: Uuid) {
    sqlx::query("UPDATE profiles SET first_pro_at = now() WHERE user_id = $1")
        .bind(user)
        .execute(pool)
        .await
        .unwrap();
}

async fn ever_pro(pool: &PgPool, user: Uuid) -> bool {
    sqlx::query_scalar::<_, Option<OffsetDateTime>>(
        "SELECT first_pro_at FROM profiles WHERE user_id = $1",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .unwrap()
    .is_some()
}

async fn status(pool: &PgPool, id: &str) -> String {
    sqlx::query_scalar("SELECT status FROM subscriptions WHERE ls_subscription_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
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

#[tokio::test]
async fn a_pass_keeps_pro_through_its_grace_month() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    let order = format!("ord_{user}");
    let now = OffsetDateTime::now_utc();
    add_row(&pool, user, &order, "once", now - Duration::days(10)).await;

    // Ten days past the end of its year: still inside the month.
    assert!(!lapse::expire_lapsed(&pool, user, STORAGE_GRACE_DAYS)
        .await
        .unwrap());
    assert_eq!(plan(&pool, user).await, "pro");
    assert_eq!(status(&pool, &order).await, "active");

    // A day past the month: gone.
    sqlx::query("UPDATE subscriptions SET renews_at = $1 WHERE ls_subscription_id = $2")
        .bind(now - Duration::days(polar::PASS_GRACE_DAYS + 1))
        .bind(&order)
        .execute(&pool)
        .await
        .unwrap();
    assert!(lapse::expire_lapsed(&pool, user, STORAGE_GRACE_DAYS)
        .await
        .unwrap());
    assert_eq!(plan(&pool, user).await, "free");
    assert_eq!(status(&pool, &order).await, "expired");

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn a_lapsed_subscription_does_not_drop_a_paid_pass() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    let now = OffsetDateTime::now_utc();
    let monthly = format!("sub_{user}");
    let late = Duration::days(lapse::RENEWAL_LATE_DAYS as i64 + 1);
    add_row(&pool, user, &monthly, "month", now - late).await;
    add_row(
        &pool,
        user,
        &format!("ord_{user}"),
        "once",
        now + Duration::days(200),
    )
    .await;

    assert!(!lapse::expire_lapsed(&pool, user, STORAGE_GRACE_DAYS)
        .await
        .unwrap());
    assert_eq!(status(&pool, &monthly).await, "expired");
    assert_eq!(
        plan(&pool, user).await,
        "pro",
        "the pass still pays for Pro"
    );

    // The daily sweep agrees and has nothing left to do for this account.
    lapse::sweep(&pool, STORAGE_GRACE_DAYS).await.unwrap();
    assert_eq!(plan(&pool, user).await, "pro");

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn renewing_in_the_grace_month_continues_the_old_year() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    // Postgres keeps microseconds; a nanosecond `now` would never compare equal
    // to what comes back out.
    let now = OffsetDateTime::now_utc();
    let now = now.replace_microsecond(now.microsecond()).unwrap();
    let first = format!("ord1_{user}");
    let second = format!("ord2_{user}");
    let year_end = now - Duration::days(25);
    add_row(&pool, user, &first, "once", year_end).await;

    let end = polar::pass_end_for(&pool, user, &second, now)
        .await
        .unwrap();
    assert_eq!(end, year_end + Duration::days(365));

    // What the webhook does next: write the new pass, retire the old one.
    add_row(&pool, user, &second, "once", end).await;
    assert_eq!(
        polar::supersede_passes(&pool, user, &second).await.unwrap(),
        1
    );
    assert_eq!(status(&pool, &first).await, "expired");
    assert_eq!(status(&pool, &second).await, "active");

    // A retry of the same order keeps its date, though the pass it continued
    // is no longer there to be found.
    let retried = polar::pass_end_for(&pool, user, &second, now + Duration::hours(6))
        .await
        .unwrap();
    assert_eq!(retried, end);

    assert!(!lapse::expire_lapsed(&pool, user, STORAGE_GRACE_DAYS)
        .await
        .unwrap());
    assert_eq!(plan(&pool, user).await, "pro");

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn a_refund_takes_pro_away_from_a_pass() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    mark_ever_pro(&pool, user).await;
    let order = format!("ord_{user}");
    add_row(
        &pool,
        user,
        &order,
        "once",
        OffsetDateTime::now_utc() + Duration::days(300),
    )
    .await;

    let revoked = lapse::revoke_refunded(&pool, &order, STORAGE_GRACE_DAYS, true)
        .await
        .unwrap();
    // Its only payment went back, so the devices kept for life go with it.
    assert_eq!(
        revoked,
        Some(Revoked {
            user_id: user,
            dropped: true,
            devices_revoked: true
        })
    );
    assert_eq!(plan(&pool, user).await, "free");
    assert_eq!(status(&pool, &order).await, "expired");
    assert!(!ever_pro(&pool, user).await);

    // A refund of something this server never granted changes nothing.
    assert_eq!(
        lapse::revoke_refunded(&pool, "ord_nobody", STORAGE_GRACE_DAYS, true)
            .await
            .unwrap(),
        None
    );

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn a_refunded_subscription_stays_refunded_until_a_new_period() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    use time::format_description::well_known::Rfc3339;
    let user = seed_pro(&pool).await;
    mark_ever_pro(&pool, user).await;
    let sub = format!("sub_{user}");
    let now = OffsetDateTime::now_utc();
    let period_end = now.replace_microsecond(now.microsecond()).unwrap() + Duration::days(20);
    add_row(&pool, user, &sub, "month", period_end).await;

    // A renewal refunded: the months before it were paid, the devices stay.
    assert_eq!(
        lapse::revoke_refunded(&pool, &sub, STORAGE_GRACE_DAYS, false)
            .await
            .unwrap(),
        Some(Revoked {
            user_id: user,
            dropped: true,
            devices_revoked: false
        })
    );
    assert_eq!(plan(&pool, user).await, "free");
    assert!(ever_pro(&pool, user).await);

    // Cancelling in Polar sends the same period back as `active`, then as
    // `grace`: neither may undo the refund.
    let same = period_end.format(&Rfc3339).unwrap();
    assert!(polar::refund_holds(&pool, &sub, "active", Some(&same))
        .await
        .unwrap());
    assert!(polar::refund_holds(&pool, &sub, "grace", Some(&same))
        .await
        .unwrap());

    // A new month paid is a new payment.
    let next = (period_end + Duration::days(30)).format(&Rfc3339).unwrap();
    assert!(!polar::refund_holds(&pool, &sub, "active", Some(&next))
        .await
        .unwrap());
    let cleared: Option<OffsetDateTime> =
        sqlx::query_scalar("SELECT refunded_at FROM subscriptions WHERE ls_subscription_id = $1")
            .bind(&sub)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(cleared, None);

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn a_refund_keeps_the_devices_of_an_account_that_paid_before() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    mark_ever_pro(&pool, user).await;
    let now = OffsetDateTime::now_utc();
    // Last year's pass, paid and used up, then this year's, refunded.
    let old = format!("ord1_{user}");
    let new = format!("ord2_{user}");
    add_row(&pool, user, &old, "once", now - Duration::days(40)).await;
    sqlx::query("UPDATE subscriptions SET status = 'expired' WHERE ls_subscription_id = $1")
        .bind(&old)
        .execute(&pool)
        .await
        .unwrap();
    add_row(&pool, user, &new, "once", now + Duration::days(325)).await;

    let revoked = lapse::revoke_refunded(&pool, &new, STORAGE_GRACE_DAYS, true)
        .await
        .unwrap()
        .unwrap();
    assert!(revoked.dropped);
    assert!(!revoked.devices_revoked, "last year was paid for");
    assert!(ever_pro(&pool, user).await);

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn a_subscription_is_not_dropped_on_its_renewal_date() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    let sub = format!("sub_{user}");
    let now = OffsetDateTime::now_utc();
    // The period ended a minute ago and Polar's webhook with the next one has
    // not landed yet: the account is still paying.
    add_row(&pool, user, &sub, "month", now - Duration::minutes(1)).await;
    assert!(!lapse::expire_lapsed(&pool, user, STORAGE_GRACE_DAYS)
        .await
        .unwrap());
    assert_eq!(status(&pool, &sub).await, "active");
    assert_eq!(plan(&pool, user).await, "pro");

    // A week on with no word from Polar, it is over.
    sqlx::query("UPDATE subscriptions SET renews_at = $1 WHERE ls_subscription_id = $2")
        .bind(now - Duration::days(lapse::RENEWAL_LATE_DAYS as i64 + 1))
        .bind(&sub)
        .execute(&pool)
        .await
        .unwrap();
    assert!(lapse::expire_lapsed(&pool, user, STORAGE_GRACE_DAYS)
        .await
        .unwrap());
    assert_eq!(plan(&pool, user).await, "free");

    // A cancelled one ends on its date, no week added.
    let cancelled = seed_pro(&pool).await;
    let sub = format!("sub_{cancelled}");
    add_row(&pool, cancelled, &sub, "month", now - Duration::minutes(1)).await;
    sqlx::query("UPDATE subscriptions SET cancel_at = renews_at WHERE ls_subscription_id = $1")
        .bind(&sub)
        .execute(&pool)
        .await
        .unwrap();
    assert!(lapse::expire_lapsed(&pool, cancelled, STORAGE_GRACE_DAYS)
        .await
        .unwrap());

    cleanup(&pool, user).await;
    cleanup(&pool, cancelled).await;
}

/// A pass and the one bought before it ran out, as the webhook leaves them: the
/// new one ends a year after the old one, and the old one is retired.
async fn chained_passes(pool: &PgPool, user: Uuid, now: OffsetDateTime) -> (String, String) {
    let first = format!("ord1_{user}");
    let second = format!("ord2_{user}");
    add_row(pool, user, &first, "once", now + Duration::days(60)).await;
    let end = polar::pass_end_for(pool, user, &second, now).await.unwrap();
    add_row(pool, user, &second, "once", end).await;
    polar::supersede_passes(pool, user, &second).await.unwrap();
    (first, second)
}

async fn renews_at(pool: &PgPool, id: &str) -> OffsetDateTime {
    sqlx::query_scalar("SELECT renews_at FROM subscriptions WHERE ls_subscription_id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn refunding_an_early_renewal_gives_the_paid_year_back() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    use time::format_description::well_known::Rfc3339;
    let user = seed_pro(&pool).await;
    mark_ever_pro(&pool, user).await;
    let now = OffsetDateTime::now_utc();
    let now = now.replace_microsecond(now.microsecond()).unwrap();
    let (first, second) = chained_passes(&pool, user, now).await;
    assert_eq!(status(&pool, &first).await, "expired");

    let revoked = lapse::revoke_refunded(&pool, &second, STORAGE_GRACE_DAYS, true)
        .await
        .unwrap()
        .unwrap();
    assert!(
        !revoked.dropped,
        "the first year still has two months to run"
    );
    assert!(!revoked.devices_revoked);
    assert_eq!(status(&pool, &first).await, "active");
    assert_eq!(renews_at(&pool, &first).await, now + Duration::days(60));
    assert_eq!(plan(&pool, user).await, "pro");
    assert_eq!(
        lapse::standing(&pool, user).await.unwrap().map(|s| s.1),
        Some(Some(now + Duration::days(60)))
    );

    // A late retry of the refunded order's `order.paid` doesn't bring it back.
    let end = polar::pass_end_for(&pool, user, &second, now)
        .await
        .unwrap();
    let end = end.format(&Rfc3339).unwrap();
    assert!(polar::refund_holds(&pool, &second, "active", Some(&end))
        .await
        .unwrap());

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn refunding_the_older_pass_takes_its_time_off_the_newer() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    let now = OffsetDateTime::now_utc();
    let now = now.replace_microsecond(now.microsecond()).unwrap();
    let (first, second) = chained_passes(&pool, user, now).await;
    assert_eq!(
        renews_at(&pool, &second).await,
        now + Duration::days(60 + 365)
    );

    assert!(
        !lapse::revoke_refunded(&pool, &first, STORAGE_GRACE_DAYS, true)
            .await
            .unwrap()
            .unwrap()
            .dropped
    );
    // A year from its own payment, the two months it carried over are gone.
    let bought: OffsetDateTime =
        sqlx::query_scalar("SELECT created_at FROM subscriptions WHERE ls_subscription_id = $1")
            .bind(&second)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        renews_at(&pool, &second).await,
        bought + Duration::days(365)
    );
    assert_eq!(status(&pool, &second).await, "active");
    assert_eq!(plan(&pool, user).await, "pro");

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn the_account_shows_the_row_that_goes_on_paying() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    let now = OffsetDateTime::now_utc();
    // A pass bought, then the monthly plan cancelled: the cancellation is the
    // newest write, and the pass is still what the account is on.
    add_row(
        &pool,
        user,
        &format!("ord_{user}"),
        "once",
        now + Duration::days(365),
    )
    .await;
    let monthly = format!("sub_{user}");
    add_row(&pool, user, &monthly, "month", now + Duration::days(12)).await;
    sqlx::query(
        "UPDATE subscriptions SET status = 'grace', cancel_at = renews_at, updated_at = now() + interval '1 second'
          WHERE ls_subscription_id = $1",
    )
    .bind(&monthly)
    .execute(&pool)
    .await
    .unwrap();
    let shown = lapse::standing(&pool, user).await.unwrap().unwrap();
    assert_eq!(shown.3, "once");
    assert_eq!(shown.2, None);

    cleanup(&pool, user).await;
}

#[tokio::test]
async fn one_paid_plan_at_a_time() {
    let Some(pool) = pool().await else {
        eprintln!("HOARD_PG_TEST_URL unset — skipping");
        return;
    };
    let user = seed_pro(&pool).await;
    let now = OffsetDateTime::now_utc();
    let monthly = format!("sub_{user}");
    add_row(&pool, user, &monthly, "month", now + Duration::days(12)).await;
    for interval in ["month", "year", "once"] {
        assert!(
            polar::already_paying(&pool, user, interval).await.unwrap(),
            "{interval}"
        );
    }
    // Cancelled, it bills no more: anything may follow it.
    sqlx::query("UPDATE subscriptions SET cancel_at = renews_at WHERE ls_subscription_id = $1")
        .bind(&monthly)
        .execute(&pool)
        .await
        .unwrap();
    assert!(!polar::already_paying(&pool, user, "once").await.unwrap());

    // On a pass, only the next pass, until its year is over.
    let on_pass = seed_pro(&pool).await;
    let order = format!("ord_{on_pass}");
    add_row(&pool, on_pass, &order, "once", now + Duration::days(100)).await;
    assert!(!polar::already_paying(&pool, on_pass, "once").await.unwrap());
    assert!(polar::already_paying(&pool, on_pass, "month")
        .await
        .unwrap());
    sqlx::query("UPDATE subscriptions SET renews_at = $1 WHERE ls_subscription_id = $2")
        .bind(now - Duration::days(3))
        .bind(&order)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !polar::already_paying(&pool, on_pass, "month")
            .await
            .unwrap(),
        "in its grace month"
    );

    cleanup(&pool, user).await;
    cleanup(&pool, on_pass).await;
}
