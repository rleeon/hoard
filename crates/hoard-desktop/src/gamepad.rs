//! Game controllers on Linux, read here and handed to the window as `gamepad`
//! events.
//!
//! The arrows drive the app (`ui/src/lib/actions/spatialNav.ts`), and Steam
//! turns the Deck's controls into those keys. A controller Steam is not
//! translating (one plugged into a PC) sends no keys at all, and Ubuntu's
//! WebKitGTK is built without the Gamepad API, so the page cannot read it
//! either. It is read here, straight from `/dev/input`, where the system already
//! lets the logged-in user read a controller (and only a controller: keyboards
//! are not theirs to read). WebView2 and macOS's engine have the Gamepad API,
//! and there the page reads the controller itself (`ui/src/lib/actions/gamepad.ts`).
//!
//! Only what navigating needs goes across: the d-pad and the left stick as four
//! directions, the bottom and right face buttons, and the two bumpers.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use evdev::{AbsoluteAxisCode, Device, EventSummary, KeyCode};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// How far the stick has to lean to count as a press, and how far back to
/// count as let go: apart, so a stick resting near the line does not chatter.
const STICK_ON: f32 = 0.6;
const STICK_OFF: f32 = 0.35;

const DIRECTIONS: [&str; 4] = ["up", "down", "left", "right"];

#[derive(Clone, Serialize)]
struct PadEvent {
    /// `up`, `down`, `left`, `right`, `a` (bottom), `b` (right), `lb`, `rb`.
    button: &'static str,
    pressed: bool,
    /// Whose labels to show: `xbox`, `playstation`, `nintendo` or `steam`.
    family: &'static str,
    pad: usize,
}

pub fn spawn(app: AppHandle) {
    let started = std::thread::Builder::new()
        .name("gamepad".into())
        .spawn(move || watch(app));
    if let Err(e) = started {
        tracing::warn!(error = %e, "couldn't start the gamepad reader");
    }
}

/// Looks for new controllers every two seconds, so one plugged in later is
/// picked up, and gives each its own reader.
fn watch(app: AppHandle) {
    let open: Arc<Mutex<HashSet<PathBuf>>> = Arc::default();
    let next_pad = AtomicUsize::new(0);
    loop {
        if let Ok(dir) = std::fs::read_dir("/dev/input") {
            for entry in dir.flatten() {
                let path = entry.path();
                let is_event = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("event"));
                if !is_event || open.lock().map_or(true, |o| o.contains(&path)) {
                    continue;
                }
                // Most nodes are not ours to open (keyboards, the mouse): that
                // failure is the normal case, not an error.
                let Ok(device) = Device::open(&path) else { continue };
                if !is_controller(&device) {
                    continue;
                }
                tracing::info!(name = device.name().unwrap_or("?"), "controller found");
                if let Ok(mut o) = open.lock() {
                    o.insert(path.clone());
                }
                let pad = next_pad.fetch_add(1, Ordering::Relaxed);
                let (app, open) = (app.clone(), open.clone());
                let _ = std::thread::Builder::new()
                    .name("gamepad-read".into())
                    .spawn(move || {
                        read(&app, device, pad);
                        if let Ok(mut o) = open.lock() {
                            o.remove(&path);
                        }
                    });
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// A controller has the face buttons: the bottom one is what a gamepad
/// announces itself by (`BTN_GAMEPAD` is the same code).
fn is_controller(device: &Device) -> bool {
    device
        .supported_keys()
        .is_some_and(|keys| keys.contains(KeyCode::BTN_SOUTH))
}

/// Reads one controller until it is unplugged.
fn read(app: &AppHandle, mut device: Device, pad: usize) {
    let family = family(device.input_id().vendor());
    let mut range_x = (-32768, 32767);
    let mut range_y = (-32768, 32767);
    if let Ok(axes) = device.get_absinfo() {
        for (axis, info) in axes {
            if axis == AbsoluteAxisCode::ABS_X {
                range_x = (info.minimum(), info.maximum());
            } else if axis == AbsoluteAxisCode::ABS_Y {
                range_y = (info.minimum(), info.maximum());
            }
        }
    }
    let mut dirs = Directions::default();
    let send = |button: &'static str, pressed: bool| {
        // A press only while the window is in front: the same controller is
        // playing a game the rest of the time. Releases always, so a held
        // direction never keeps repeating.
        if pressed && !focused(app) {
            return;
        }
        let _ = app.emit_to(
            "main",
            "gamepad",
            PadEvent {
                button,
                pressed,
                family,
                pad,
            },
        );
    };
    loop {
        let Ok(events) = device.fetch_events() else {
            tracing::info!("controller gone");
            // Unplugged, or out of battery, with something held: the page
            // would have kept repeating it with nothing left to let go.
            for button in DIRECTIONS.into_iter().chain(["a", "b", "lb", "rb"]) {
                send(button, false);
            }
            return;
        };
        for ev in events {
            match ev.destructure() {
                // 2 is the kernel's own autorepeat: the page repeats by itself.
                EventSummary::Key(_, _, 2) => {}
                EventSummary::Key(_, code, value) => {
                    if let Some(i) = dpad_index(code) {
                        dirs.set(DPAD, i, value != 0, &send);
                    } else if let Some(name) = button_name(code) {
                        send(name, value != 0);
                    }
                }
                EventSummary::AbsoluteAxis(_, code, value) => match code {
                    AbsoluteAxisCode::ABS_HAT0Y => {
                        dirs.set(HAT, 0, value < 0, &send);
                        dirs.set(HAT, 1, value > 0, &send);
                    }
                    AbsoluteAxisCode::ABS_HAT0X => {
                        dirs.set(HAT, 2, value < 0, &send);
                        dirs.set(HAT, 3, value > 0, &send);
                    }
                    AbsoluteAxisCode::ABS_X => {
                        dirs.stick(2, normalize(value, range_x), &send);
                    }
                    // Y grows downwards.
                    AbsoluteAxisCode::ABS_Y => {
                        dirs.stick(0, normalize(value, range_y), &send);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

/// What holds each direction down: a controller can draw its d-pad as buttons
/// or as a hat, and the stick leans too. A direction is pressed while any of
/// them holds it, so moving between them never sends it twice.
const DPAD: u8 = 1;
const HAT: u8 = 2;
const STICK: u8 = 4;

#[derive(Default)]
struct Directions([u8; 4]);

impl Directions {
    fn set(&mut self, source: u8, i: usize, on: bool, send: &impl Fn(&'static str, bool)) {
        let before = self.0[i] != 0;
        if on {
            self.0[i] |= source;
        } else {
            self.0[i] &= !source;
        }
        let after = self.0[i] != 0;
        if before != after {
            send(DIRECTIONS[i], after);
        }
    }

    /// `first` is the negative end of the axis (up, or left), `first + 1` the
    /// positive one; `v` runs from -1 to 1.
    fn stick(&mut self, first: usize, v: f32, send: &impl Fn(&'static str, bool)) {
        for (i, lean) in [(first, -v), (first + 1, v)] {
            let held = self.0[i] & STICK != 0;
            let on = if held { lean > STICK_OFF } else { lean > STICK_ON };
            if on != held {
                self.set(STICK, i, on, send);
            }
        }
    }
}

fn normalize(value: i32, (min, max): (i32, i32)) -> f32 {
    if max <= min {
        return 0.0;
    }
    (2.0 * (value - min) as f32 / (max - min) as f32) - 1.0
}

fn dpad_index(code: KeyCode) -> Option<usize> {
    Some(match code {
        KeyCode::BTN_DPAD_UP => 0,
        KeyCode::BTN_DPAD_DOWN => 1,
        KeyCode::BTN_DPAD_LEFT => 2,
        KeyCode::BTN_DPAD_RIGHT => 3,
        _ => return None,
    })
}

fn button_name(code: KeyCode) -> Option<&'static str> {
    Some(match code {
        // By position, not by what is printed on it: bottom goes in, right goes
        // back, on every brand (Nintendo prints B and A there).
        KeyCode::BTN_SOUTH => "a",
        KeyCode::BTN_EAST => "b",
        KeyCode::BTN_TL => "lb",
        KeyCode::BTN_TR => "rb",
        _ => return None,
    })
}

fn family(vendor: u16) -> &'static str {
    match vendor {
        0x054c => "playstation",
        0x057e => "nintendo",
        0x28de => "steam",
        _ => "xbox",
    }
}

fn focused(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .and_then(|w| w.is_focused().ok())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn a_direction_held_by_two_sources_is_sent_once() {
        let log = RefCell::new(Vec::new());
        let send = |b, p| log.borrow_mut().push((b, p));
        let mut d = Directions::default();
        d.set(HAT, 0, true, &send);
        d.stick(0, -0.9, &send);
        d.set(HAT, 0, false, &send);
        d.stick(0, 0.0, &send);
        assert_eq!(*log.borrow(), vec![("up", true), ("up", false)]);
    }

    #[test]
    fn the_stick_lets_go_only_well_short_of_where_it_pressed() {
        let log = RefCell::new(Vec::new());
        let send = |b, p| log.borrow_mut().push((b, p));
        let mut d = Directions::default();
        d.stick(2, 0.5, &send);
        assert!(log.borrow().is_empty());
        d.stick(2, 0.7, &send);
        d.stick(2, 0.45, &send);
        assert_eq!(*log.borrow(), vec![("right", true)]);
        d.stick(2, 0.2, &send);
        assert_eq!(*log.borrow(), vec![("right", true), ("right", false)]);
    }

    #[test]
    fn normalize_spans_minus_one_to_one() {
        assert_eq!(normalize(0, (0, 255)), -1.0);
        assert_eq!(normalize(255, (0, 255)), 1.0);
        assert!(normalize(128, (0, 255)).abs() < 0.01);
    }
}
