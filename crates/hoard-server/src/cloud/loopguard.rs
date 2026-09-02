//! Brakes for clients that keep asking for the same thing forever.
//!
//! Two incidents, one shape. In both a client asks for something that will
//! never change, gets a perfectly successful answer, learns nothing from it,
//! and asks again, and nothing on either side counts how many times that has
//! happened.
//!
//! **The restore loop.** A user's Windows client pulled the same 2.83 MB
//! version of one save *once a minute* from 27-jul to 3-ago-2026: 3.752
//! downloads, 10,6 GB. The bug is in the client (a restore that writes nothing
//! leaves the folder empty, and an empty folder is what triggers the restore),
//! and it is fixed there. But it was found by chance while sweeping Fly logs
//! eight days in, and the only way to stop it that afternoon was an `UPDATE
//! saves SET backup_only = true` typed by hand against production, which hid
//! that person's only cloud copy from their only machine. That row stays set:
//! it was reviewed on 2-sep-2026 and kept on purpose, so a sweep that finds it
//! should leave it alone. The same shape shows up at gigabyte scale:
//! 111 pulls of one 1,57 GB version in a fortnight, 170 GB.
//!
//! **The full-account loop.** When an account is over quota the upload is
//! refused at init with a 402, before any bytes move: cheap, correct, and
//! completely ignored by the client, which tries the next save, and the next,
//! and comes back a minute later. One account collected 342 refusals in three
//! hours; another 148 in a day. The client-side fix (park the whole account for
//! an hour on 402) landed after v1.1.2, so *every* client in the wild today
//! still hammers.
//!
//! ## Why this lives on the server
//!
//! Because the client that's looping is, by definition, the one you can't
//! patch. A fix shipped in the next release protects the people who update;
//! this protects the ones who don't, from the day it deploys. And it needs no
//! new client contract: v1.1.2 already maps *any* 429 to `ApiError::RateLimited`
//! (reading `retry_after_seconds` from the body, then the `Retry-After` header,
//! defaulting to 60s) and honours it in both the backup and the auto-restore
//! path. A brake expressed as a 429 is understood by everything already
//! installed. Even a client that ignores it costs a rejected request instead of
//! a gigabyte.
//!
//! ## Why pacing and not refusing
//!
//! Re-restoring the same version is a legitimate thing to do: testing a save,
//! hopping between machines, undoing a bad session. So the download brake never
//! says no, it says *later*, with the gap widening as the count climbs. A
//! person restoring by hand crosses the first threshold about never; a loop
//! crosses all of them by lunchtime and settles at one pull a day.
//!
//! There is also nothing to remember and nothing to clean up, which is the
//! whole point of doing it here rather than with another hand-typed UPDATE: the
//! counter is keyed by version number, so the moment a new version is uploaded
//! the key changes and the brake is gone by construction.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use uuid::Uuid;

use crate::cloud::state::CloudState;

/// Downloads of the *same* (user, save, version) inside 24h before the server
/// starts pacing them.
///
/// Eight is generous on purpose. The observability threshold that only writes a
/// WARN sits at five, and that one is allowed to cry wolf; this one changes
/// what the user gets, so it wants headroom above any believable human. Three
/// machines each restoring the same version twice is six, and still passes.
/// The loop it exists for reached 536 in a day.
const DOWNLOAD_FREE_PER_DAY: i64 = 8;

/// Quota refusals for one account inside an hour before the refusal starts
/// carrying a wait.
///
/// A full account with a dozen tracked games legitimately produces one refusal
/// per save on the first sweep after it fills up, so the threshold has to clear
/// a whole library. Five is below that on purpose: they arrive in a burst
/// within seconds of each other, and pacing from the fifth onward turns "342
/// refusals in three hours" into "5, then one an hour".
const QUOTA_BLOCKS_FREE_PER_HOUR: i64 = 5;

/// How long a paced account is told to wait.
///
/// One hour, flat, matching the client-side park that shipped after v1.1.2. If
/// both ends are running the same number, an updated client and an old one
/// behave the same. Deliberately *not* escalating: the account stops being full
/// the moment somebody deletes a save or upgrades, and a six-hour wait would
/// leave them staring at a Hoard that refuses to back up something it would now
/// happily accept.
const QUOTA_WAIT_SECS: i64 = 3600;

/// A brake that fired: how long the client is asked to wait, and the count that
/// tripped it (carried into the response body and the log line, because "you
/// are being paced" is useless to a support conversation without the number).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pace {
    pub retry_after_secs: i64,
    pub seen: i64,
}

/// Minimum gap the server allows between two downloads of the same version,
/// given how many it has already served in the last 24h. `None` = no pacing.
///
/// The bands are wide and few because the thing being throttled is not a rate,
/// it's a belief: a client that pulls the same immutable bytes for the ninth
/// time today is wrong, and it will still be wrong in an hour. Once it has been
/// wrong two dozen times, once a day is as much benefit of the doubt as the
/// evidence supports.
///
/// The first band is the gentle one on purpose. Replaying 60 days of
/// `sync_log` through these numbers, 179 (user, save, version, day) groups
/// would have been paced across 11 accounts, but 19 of those groups, from two
/// accounts, carry 9.105 of the 11.393 paced downloads. That is the shape of
/// the whole problem: a couple of runaway clients and a tail of people who
/// genuinely restored the same version nine or ten times in an afternoon. The
/// tail gets a quarter of an hour, which nobody will notice on a restore they
/// asked for; the runaways cross into the second band within two hours and the
/// third by evening.
pub fn download_gap_secs(seen_24h: i64) -> Option<i64> {
    match seen_24h {
        n if n < DOWNLOAD_FREE_PER_DAY => None,
        n if n < 16 => Some(900),
        n if n < 24 => Some(6 * 3600),
        _ => Some(24 * 3600),
    }
}

/// Decide whether this download waits. Pure, so the arithmetic is testable
/// without a database or a clock.
///
/// `since_last_secs` is how long ago the last *served* download of this version
/// was; `None` means there is no previous one (and then there is nothing to
/// pace against). Refused attempts never reach this, since they write no
/// `download` row, so the gap is measured between bytes actually handed out,
/// not between attempts, and a hammering client can't push its own window
/// forward by hammering.
pub fn download_pace(seen_24h: i64, since_last_secs: Option<i64>) -> Option<Pace> {
    let gap = download_gap_secs(seen_24h)?;
    let elapsed = since_last_secs?;
    let wait = gap - elapsed;
    (wait > 0).then_some(Pace {
        retry_after_secs: wait,
        seen: seen_24h,
    })
}

/// Decide whether a quota refusal carries a wait.
pub fn quota_pace(blocks_last_hour: i64) -> Option<Pace> {
    (blocks_last_hour >= QUOTA_BLOCKS_FREE_PER_HOUR).then_some(Pace {
        retry_after_secs: QUOTA_WAIT_SECS,
        seen: blocks_last_hour,
    })
}

/// Ask the log whether this download should wait.
///
/// Counts the rows written *before* the one this request would write, so the
/// first call after the threshold sees exactly the threshold. Backed by
/// `idx_sync_log_repeat_download` (migration 0034), the same index the WARN
/// query uses.
///
/// Fails open, loudly. This runs in front of a paid restore: if the count query
/// times out, serving one extra download is a far better outcome than refusing
/// a restore because an accounting query was slow.
pub async fn download_brake(
    state: &CloudState,
    user_id: Uuid,
    save_id: &str,
    version: i64,
) -> Option<Pace> {
    let row: Result<(i64, Option<f64>), _> = sqlx::query_as(
        "SELECT count(*),
                extract(epoch FROM now() - max(at))
           FROM sync_log
          WHERE user_id = $1 AND save_id = $2 AND version_num = $3
            AND kind = 'download' AND at > now() - interval '24 hours'",
    )
    .bind(user_id)
    .bind(save_id)
    .bind(version)
    .fetch_one(&state.pool)
    .await;

    match row {
        Ok((seen, since)) => download_pace(seen, since.map(|s| s as i64)),
        Err(e) => {
            tracing::warn!(error = %e, %user_id, %save_id, version, "loopguard: repeat-download count failed; serving anyway");
            None
        }
    }
}

/// Ask the log whether this account's quota refusals should start carrying a
/// wait. Same fail-open contract as [`download_brake`].
pub async fn quota_brake(state: &CloudState, user_id: Uuid) -> Option<Pace> {
    let row: Result<(i64,), _> = sqlx::query_as(
        "SELECT count(*) FROM sync_log
          WHERE user_id = $1 AND kind = 'quota_block'
            AND at > now() - interval '1 hour'",
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await;

    match row {
        Ok((blocks,)) => quota_pace(blocks),
        Err(e) => {
            tracing::warn!(error = %e, %user_id, "loopguard: quota-block count failed; answering 402 as usual");
            None
        }
    }
}

/// The 429 a paced client gets.
///
/// `detail` carries whatever the un-paced answer would have said (the quota
/// figures, for the full-account case) so the window can still explain *why*
/// instead of showing a bare "rate limited". `retry_after_seconds` is in the
/// body as well as the header because that's the order the client reads them
/// in.
#[derive(Debug, Serialize)]
pub struct PacedResponse<T: Serialize> {
    pub error: &'static str,
    pub code: &'static str,
    pub retry_after_seconds: i64,
    /// What tripped the brake: the 24h download count, or the refusals seen
    /// this hour. Named for a human reading a support ticket.
    pub repeated: i64,
    #[serde(flatten)]
    pub detail: T,
}

impl<T: Serialize> IntoResponse for PacedResponse<T> {
    fn into_response(self) -> Response {
        let retry_after = self.retry_after_seconds.clamp(1, 86_400).to_string();
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::RETRY_AFTER, retry_after)],
            axum::Json(self),
        )
            .into_response()
    }
}

/// The paced answer to a restore that keeps coming back.
pub fn restore_loop_response(pace: Pace) -> Response {
    PacedResponse {
        error: "this version was already downloaded repeatedly; the server is spacing out retries",
        code: "restore_paced",
        retry_after_seconds: pace.retry_after_secs,
        repeated: pace.seen,
        detail: serde_json::json!({}),
    }
    .into_response()
}

/// The paced answer to an account that keeps trying to upload while full.
pub fn quota_paced_response<T: Serialize>(pace: Pace, quota_detail: T) -> Response {
    PacedResponse {
        error: "storage quota exceeded; retries are being spaced out",
        code: "quota_exceeded_paced",
        retry_after_seconds: pace.retry_after_secs,
        repeated: pace.seen,
        detail: quota_detail,
    }
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handful_of_restores_a_day_is_never_paced() {
        for seen in 0..DOWNLOAD_FREE_PER_DAY {
            assert_eq!(download_gap_secs(seen), None, "seen = {seen}");
            assert_eq!(download_pace(seen, Some(1)), None, "seen = {seen}");
        }
    }

    #[test]
    fn the_gap_widens_as_the_count_climbs() {
        assert_eq!(download_gap_secs(8), Some(900));
        assert_eq!(download_gap_secs(15), Some(900));
        assert_eq!(download_gap_secs(16), Some(6 * 3600));
        assert_eq!(download_gap_secs(23), Some(6 * 3600));
        assert_eq!(download_gap_secs(24), Some(24 * 3600));
        assert_eq!(download_gap_secs(3752), Some(24 * 3600));
    }

    /// The wait handed to the client is what's *left* of the gap, not the whole
    /// gap: a client that comes back 50 minutes into an hour-long gap waits ten
    /// minutes. Getting this backwards would turn a paced client into a stalled
    /// one, which is the failure the band-aid this replaces already had.
    #[test]
    fn the_wait_is_the_remainder_of_the_gap() {
        assert_eq!(
            download_pace(8, Some(600)),
            Some(Pace {
                retry_after_secs: 300,
                seen: 8
            })
        );
    }

    /// Once the gap has passed, the download goes through: the brake paces,
    /// it never latches. A version nobody has pulled in a day is served on the
    /// first ask no matter how ugly its history.
    #[test]
    fn waiting_out_the_gap_serves_the_download() {
        assert_eq!(download_pace(9, Some(900)), None);
        assert_eq!(download_pace(3752, Some(24 * 3600 + 1)), None);
    }

    /// No previous download means nothing to pace against. Can only happen if
    /// the count and the timestamp disagree, but the arithmetic shouldn't
    /// invent a wait out of a NULL.
    #[test]
    fn no_previous_download_is_not_paced() {
        assert_eq!(download_pace(99, None), None);
    }

    #[test]
    fn a_full_library_hitting_the_wall_once_is_not_paced() {
        for blocks in 0..QUOTA_BLOCKS_FREE_PER_HOUR {
            assert_eq!(quota_pace(blocks), None, "blocks = {blocks}");
        }
        assert_eq!(
            quota_pace(QUOTA_BLOCKS_FREE_PER_HOUR),
            Some(Pace {
                retry_after_secs: QUOTA_WAIT_SECS,
                seen: QUOTA_BLOCKS_FREE_PER_HOUR
            })
        );
    }

    /// A 429 is only useful to the shipped client if the header is there: the
    /// v1.1.2 parser reads `retry_after_seconds` from the body first and falls
    /// back to `Retry-After`, and a self-hosted or third-party client may only
    /// know the header.
    #[test]
    fn the_paced_response_carries_the_header() {
        let resp = restore_loop_response(Pace {
            retry_after_secs: 900,
            seen: 12,
        });
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            resp.headers().get(header::RETRY_AFTER).unwrap(),
            "900",
            "the client falls back to this header when the body has no hint"
        );
    }

    /// A day-long gap still has to fit in the header, and a negative or absurd
    /// value must never reach it.
    #[test]
    fn the_header_is_clamped_to_something_sane() {
        let resp = restore_loop_response(Pace {
            retry_after_secs: -5,
            seen: 12,
        });
        assert_eq!(resp.headers().get(header::RETRY_AFTER).unwrap(), "1");
        let resp = restore_loop_response(Pace {
            retry_after_secs: 999_999,
            seen: 12,
        });
        assert_eq!(resp.headers().get(header::RETRY_AFTER).unwrap(), "86400");
    }
}
