//! A copy of the Supabase Auth account list in `auth.users`, for the admin
//! metrics.
//!
//! On Supabase that table is the real one and this stops at its first look. In
//! our own Postgres it is a stand-in (`deploy/fly/pg/supabase-stub.sql`) and
//! `admin_metrics` would count zero accounts, zero unconfirmed, zero per
//! provider. So every hour, and once at start-up so a freshly restored database
//! does not wait an hour, the server reads the list off the Admin API and
//! writes it down.

use crate::cloud::{state::CloudState, supabase_admin};
use sqlx::PgConnection;
use std::time::Duration;

const EVERY: Duration = Duration::from_secs(3600);

pub fn spawn(state: CloudState) {
    let Some(cloud) = state.config.cloud.clone() else {
        return;
    };
    if cloud.supabase_url.is_empty() || cloud.supabase_service_role_key.is_empty() {
        tracing::info!("auth mirror: no Supabase admin credentials, not copying accounts");
        return;
    }
    tokio::spawn(async move {
        let http = match reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
        {
            Ok(h) => h,
            Err(e) => {
                tracing::warn!(error = %e, "auth mirror: no http client");
                return;
            }
        };
        let mut tick = tokio::time::interval(EVERY);
        loop {
            tick.tick().await;
            match supabase_owns_auth(&state.pool).await {
                Ok(true) => {
                    tracing::info!("auth mirror: auth.users is Supabase's own, nothing to copy");
                    return;
                }
                Ok(false) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "auth mirror: could not inspect auth.users");
                    continue;
                }
            }
            let users = match supabase_admin::list_users(
                &http,
                &cloud.supabase_url,
                &cloud.supabase_service_role_key,
            )
            .await
            {
                Ok(u) => u,
                Err(e) => {
                    tracing::warn!(error = %e, "auth mirror: listing accounts failed");
                    continue;
                }
            };
            let res = async {
                let mut tx = state.pool.begin().await?;
                let counts = sync(&mut tx, &users).await?;
                tx.commit().await?;
                Ok::<_, sqlx::Error>(counts)
            }
            .await;
            match res {
                Ok((written, removed)) => {
                    tracing::info!(written, removed, "auth mirror: accounts copied")
                }
                Err(e) => tracing::warn!(error = %e, "auth mirror: writing accounts failed"),
            }
        }
    });
}

/// Supabase's table has a password column; the stand-in never will.
#[doc(hidden)]
pub async fn supabase_owns_auth(pool: &sqlx::PgPool) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM information_schema.columns
                         WHERE table_schema = 'auth' AND table_name = 'users'
                           AND column_name = 'encrypted_password')",
    )
    .fetch_one(pool)
    .await
}

/// Write `users` over the copy. Returns rows written and rows removed.
#[doc(hidden)]
pub async fn sync(
    conn: &mut PgConnection,
    users: &[supabase_admin::AdminUser],
) -> sqlx::Result<(u64, u64)> {
    let ids: Vec<uuid::Uuid> = users.iter().map(|u| u.id).collect();
    let text = |f: fn(&supabase_admin::AdminUser) -> &Option<String>| -> Vec<Option<String>> {
        users.iter().map(|u| f(u).clone()).collect()
    };
    let meta: Vec<String> = users.iter().map(|u| u.app_metadata.to_string()).collect();

    let written = sqlx::query(
        "INSERT INTO auth.users (id, email, email_confirmed_at, banned_until, raw_app_meta_data,
                                 created_at, last_sign_in_at)
         SELECT * FROM UNNEST($1::uuid[], $2::text[], $3::text[]::timestamptz[],
                              $4::text[]::timestamptz[], $5::text[]::jsonb[],
                              $6::text[]::timestamptz[], $7::text[]::timestamptz[])
         ON CONFLICT (id) DO UPDATE SET
             email = EXCLUDED.email,
             email_confirmed_at = EXCLUDED.email_confirmed_at,
             banned_until = EXCLUDED.banned_until,
             raw_app_meta_data = EXCLUDED.raw_app_meta_data,
             created_at = EXCLUDED.created_at,
             last_sign_in_at = EXCLUDED.last_sign_in_at",
    )
    .bind(&ids)
    .bind(text(|u| &u.email))
    .bind(text(|u| &u.email_confirmed_at))
    .bind(text(|u| &u.banned_until))
    .bind(&meta)
    .bind(text(|u| &u.created_at))
    .bind(text(|u| &u.last_sign_in_at))
    .execute(&mut *conn)
    .await?
    .rows_affected();

    let existing: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.users")
        .fetch_one(&mut *conn)
        .await?;
    if !safe_to_prune(users.len(), existing) {
        return Ok((written, 0));
    }
    let removed = sqlx::query("DELETE FROM auth.users WHERE NOT (id = ANY($1))")
        .bind(&ids)
        .execute(&mut *conn)
        .await?
        .rows_affected();
    Ok((written, removed))
}

/// Accounts deleted in Supabase leave the copy, but a list that comes back
/// much shorter than the copy is more likely a bad answer than a purge, and
/// the metrics would rather count a few ghosts than lose every account.
fn safe_to_prune(fetched: usize, existing: i64) -> bool {
    fetched > 0 && (fetched as i64) * 10 >= existing * 9
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_list_does_not_empty_the_copy() {
        assert!(!safe_to_prune(0, 225), "an empty answer prunes nothing");
        assert!(!safe_to_prune(100, 225));
        assert!(safe_to_prune(224, 225), "one account deleted in Supabase");
        assert!(safe_to_prune(230, 225));
        assert!(safe_to_prune(5, 0), "a first copy into an empty table");
    }
}
