//! The active session's device census, for the Eye panel.
//!
//! It exists because the road that was already there is cloud-only: `cloud_feed`
//! asks `/v1/devices` with the Supabase credentials and Realtime fires it when
//! another machine beats. A server of your own has neither of those, so it has to
//! ask on its own.
//!
//! This serves both: `current_client` picks the active session (self-hosted wins,
//! otherwise cloud) and `/v1/devices` is the same route on both deployments. The UI
//! calls this while the panel is open; with a cloud session the Realtime push keeps
//! arriving down its own road.

use hoard_agent::api::DeviceListOut;
use tauri::{AppHandle, State};

use crate::state::AppState;

#[tauri::command]
pub async fn devices_list(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DeviceListOut, String> {
    let client = crate::commands::library::current_client(&app, &state).await?;
    match client.list_devices().await {
        Ok(list) => {
            tracing::debug!(devices = list.devices.len(), "devices: listed");
            Ok(list)
        }
        // A server older than 1.1.3: the route does not exist. The panel keeps this
        // machine, as it always did, and paints no error over something the user
        // cannot fix from here.
        //
        // The 404 is asked about rather than the capability being probed first, and
        // deliberately so: `current_client` builds a fresh `ApiClient` on every call,
        // so its `/v1/health` probe is not cached and asking would cost **one extra
        // request every 15 seconds** to learn what the response already says.
        Err(e) => match e.downcast_ref::<hoard_agent::api::ApiError>() {
            Some(hoard_agent::api::ApiError::NotFound) => Ok(DeviceListOut {
                devices: Vec::new(),
            }),
            _ => Err(format!("{e:#}")),
        },
    }
}
