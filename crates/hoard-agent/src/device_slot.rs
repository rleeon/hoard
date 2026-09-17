//! Does this machine fit on the account?
//!
//! The Free plan comes with three devices and Pro with as many as you like. The
//! server counts them (`register_device` in `cloud/routes/me.rs`) but does not
//! turn anyone away, deliberately: gating uploads on a miscount would lock
//! people out of their own saves. So the answer has to be given where a person
//! is watching, which is the end of a login, and it has to be the same answer in
//! the window and in the terminal. Hence here, in the agent, and not twice in
//! the two frontends.
//!
//! What we do *not* do is punish a machine that was already on the account. A
//! handful of Free accounts are over three devices from before the limit meant
//! anything; signing in again from a computer they already had must keep
//! working. Only a machine the account has never seen is turned away.

use serde::Deserialize;

use crate::cloud_account::CloudError;

/// Off until the server enforces the same limit. The check is ready and the window
/// already has the dialog for it, but refusing a login here while the server still
/// lets the account grow would make it the app's rule, not the plan's. It goes on
/// together with the brake in `register_device`.
const ENFORCED: bool = false;

/// A device row created within this window counts as "the login just registered
/// it". Generous on purpose: the clocks are two different machines' and being
/// wrong in the other direction would block someone who belongs here.
const JUST_REGISTERED_SECS: i64 = 5 * 60;

/// The verdict for this machine.
#[derive(Debug, Clone)]
pub enum Admission {
    /// There is room, or there is no way to tell, which counts as room.
    Allowed,
    /// The account is full and this machine is new to it.
    Denied(Denial),
}

/// Why the machine was turned away, with everything the message needs.
#[derive(Debug, Clone)]
pub struct Denial {
    pub used: i64,
    pub limit: i64,
    /// `"free"` / `"pro"`, so the caller can offer the upgrade only when it buys
    /// something.
    pub plan: String,
    /// The row the server created for this machine while answering. Whoever
    /// cancels the login passes it to [`release`] so the account is not left
    /// carrying a device nobody signed into.
    pub device_id: Option<String>,
}

/// The slice of `/v1/me` this needs.
#[derive(Debug, Deserialize)]
struct MeSlice {
    #[serde(default)]
    plan: String,
    #[serde(default)]
    devices_used: i64,
    /// `-1` is unlimited.
    #[serde(default = "unlimited")]
    devices_limit: i64,
}

fn unlimited() -> i64 {
    -1
}

#[derive(Debug, Deserialize)]
struct DeviceListSlice {
    #[serde(default)]
    devices: Vec<DeviceSlice>,
}

#[derive(Debug, Deserialize)]
struct DeviceSlice {
    id: String,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    this_device: bool,
}

/// Ask the server whether this machine can join the account, right after a login
/// and before the session is handed to the service.
///
/// Errs on the side of letting people in: a server too old to report the count,
/// a network bump, an unparseable answer, all resolve to [`Admission::Allowed`].
/// A login that fails because we could not *ask* would be a worse bug than the
/// one this prevents.
pub async fn admit(base: &str, token: &str) -> Result<Admission, CloudError> {
    if !ENFORCED {
        return Ok(Admission::Allowed);
    }
    let me = fetch_me_slice(base, token).await?;
    if me.devices_limit < 0 || me.devices_used <= me.devices_limit {
        return Ok(Admission::Allowed);
    }

    // Over the limit. Whether that is this machine's fault is another question:
    // only a row the server has just created for us is.
    let devices = fetch_devices(base, token).await?;
    let Some(mine) = devices.devices.iter().find(|d| d.this_device) else {
        // The server did not recognise us (an old build, or no fingerprint to
        // send). It cannot be us who tipped the count over, then.
        return Ok(Admission::Allowed);
    };
    if !just_registered(mine.created_at.as_deref()) {
        return Ok(Admission::Allowed);
    }

    Ok(Admission::Denied(Denial {
        used: me.devices_used,
        limit: me.devices_limit,
        plan: me.plan,
        device_id: Some(mine.id.clone()),
    }))
}

/// Give back the row the check created. Best-effort: the count self-heals on the
/// next `/v1/me` from a machine that is actually signed in.
pub async fn release(base: &str, token: &str, device_id: &str) -> Result<(), CloudError> {
    let url = format!("{}/v1/devices/{device_id}", base.trim_end_matches('/'));
    let resp = client()?
        .delete(&url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(CloudError::Http {
            status: resp.status().as_u16(),
            body: resp.text().await.unwrap_or_default(),
        });
    }
    Ok(())
}

fn just_registered(created_at: Option<&str>) -> bool {
    let Some(raw) = created_at else {
        // No timestamp, no way to tell it apart from a machine that was already
        // there, so we treat it as one.
        return false;
    };
    let Ok(created) =
        time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339)
    else {
        return false;
    };
    let age = time::OffsetDateTime::now_utc() - created;
    age.whole_seconds() <= JUST_REGISTERED_SECS
}

fn client() -> Result<reqwest::Client, CloudError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(concat!("hoard-agent/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| CloudError::Network(e.to_string()))
}

/// `GET /v1/me` **with the device headers**: they are what makes the server
/// register this machine, and asking without them would count a device that
/// never announced itself.
async fn fetch_me_slice(base: &str, token: &str) -> Result<MeSlice, CloudError> {
    let dev = crate::logship::device_identity();
    let url = format!("{}/v1/me", base.trim_end_matches('/'));
    let mut req = client()?
        .get(&url)
        .bearer_auth(token)
        .header("x-hoard-device-fp", &dev.fingerprint)
        .header("x-hoard-device-os", &dev.os)
        .header("x-hoard-app-version", env!("CARGO_PKG_VERSION"));
    if let Some(name) = dev.name.as_deref() {
        req = req.header("x-hoard-device-name", name);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(CloudError::Unauthorized);
    }
    if !status.is_success() {
        return Err(CloudError::Http {
            status: status.as_u16(),
            body,
        });
    }
    serde_json::from_str(&body).map_err(|e| CloudError::Parse(format!("parsing /v1/me: {e}")))
}

async fn fetch_devices(base: &str, token: &str) -> Result<DeviceListSlice, CloudError> {
    let dev = crate::logship::device_identity();
    let url = format!("{}/v1/devices", base.trim_end_matches('/'));
    let resp = client()?
        .get(&url)
        .bearer_auth(token)
        .header("x-hoard-device-fp", &dev.fingerprint)
        .send()
        .await
        .map_err(|e| CloudError::Network(e.to_string()))?;
    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(CloudError::Http {
            status: status.as_u16(),
            body,
        });
    }
    serde_json::from_str(&body).map_err(|e| CloudError::Parse(format!("parsing /v1/devices: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rfc3339(offset_secs: i64) -> String {
        let t = time::OffsetDateTime::now_utc() + time::Duration::seconds(offset_secs);
        t.format(&time::format_description::well_known::Rfc3339)
            .expect("formats")
    }

    #[test]
    fn a_row_created_during_this_login_is_ours() {
        assert!(just_registered(Some(&rfc3339(-10))));
    }

    /// The one that protects the accounts already over three devices: an old row
    /// means the machine was here before the limit was, and it keeps its place.
    #[test]
    fn a_row_from_last_month_belongs_to_a_machine_that_was_already_here() {
        assert!(!just_registered(Some(&rfc3339(-30 * 24 * 3600))));
    }

    #[test]
    fn no_timestamp_is_treated_as_an_old_machine() {
        assert!(!just_registered(None));
        assert!(!just_registered(Some("not a date")));
    }
}
