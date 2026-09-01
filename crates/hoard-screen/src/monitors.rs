//! Monitor enumeration, the list of physical screens the overlay can target.
//!
//! A [`Scene`](crate::scene::Scene) panel carries a
//! [`PanelTarget`](crate::scene::PanelTarget) that names a monitor by **id**.
//! That id is the index into the list returned here, and the list is sorted so
//! the **primary monitor is id 0** (matching `PanelTarget`'s default). Both the
//! editor's monitor picker (via the `--list-monitors` subcommand) and the
//! Windows compositor read this same list, so the ids line up across the two.
//!
//! Only Windows enumerates real monitors today (where multi-monitor was the
//! reported pain point). Other platforms return an empty list: the editor then
//! falls back to the browser's monitor API and their single-surface runtimes
//! keep drawing every panel on the one screen, a graceful single-monitor mode.

use serde::Serialize;

/// One physical monitor in virtual-desktop coordinates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MonitorInfo {
    /// Stable index into the sorted list; `0` is always the primary monitor.
    pub id: u32,
    /// OS device name (e.g. `\\.\DISPLAY1`), for the picker label.
    pub name: String,
    /// Top-left in virtual-desktop pixels (can be negative for left-of-primary).
    pub x: i32,
    pub y: i32,
    /// Size in pixels.
    pub w: i32,
    pub h: i32,
    /// True for the OS primary monitor (the one whose top-left is virtual 0,0).
    pub primary: bool,
}

/// Enumerate monitors, primary first (id 0). Empty on platforms without a
/// native enumeration (callers treat empty as "single monitor").
#[cfg(all(target_os = "windows", feature = "runtime"))]
pub fn list_monitors() -> Vec<MonitorInfo> {
    use windows::Win32::Foundation::{BOOL, LPARAM, RECT, TRUE};
    use windows::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
    };
    use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

    struct Raw {
        name: String,
        rect: RECT,
        primary: bool,
    }

    extern "system" fn cb(hmon: HMONITOR, _hdc: HDC, _rc: *mut RECT, lp: LPARAM) -> BOOL {
        unsafe {
            let out = &mut *(lp.0 as *mut Vec<Raw>);
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(hmon, &mut mi as *mut _ as *mut MONITORINFO).as_bool() {
                let end = mi
                    .szDevice
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(mi.szDevice.len());
                out.push(Raw {
                    name: String::from_utf16_lossy(&mi.szDevice[..end]),
                    rect: mi.monitorInfo.rcMonitor,
                    primary: (mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0,
                });
            }
        }
        TRUE
    }

    let mut raw: Vec<Raw> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(cb), LPARAM(&mut raw as *mut _ as isize));
    }
    // Primary first, then left-to-right / top-to-bottom, so ids are stable and
    // id 0 is the primary (PanelTarget's default).
    raw.sort_by(|a, b| {
        b.primary
            .cmp(&a.primary)
            .then(a.rect.left.cmp(&b.rect.left))
            .then(a.rect.top.cmp(&b.rect.top))
    });
    raw.into_iter()
        .enumerate()
        .map(|(i, r)| MonitorInfo {
            id: i as u32,
            name: r.name,
            x: r.rect.left,
            y: r.rect.top,
            w: r.rect.right - r.rect.left,
            h: r.rect.bottom - r.rect.top,
            primary: r.primary,
        })
        .collect()
}

/// No native enumeration compiled in, single-monitor fallback. Covers every
/// non-Windows platform and the Windows core build without the `runtime`
/// feature (which doesn't pull in the `windows` crate).
#[cfg(not(all(target_os = "windows", feature = "runtime")))]
pub fn list_monitors() -> Vec<MonitorInfo> {
    Vec::new()
}
