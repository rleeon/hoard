//! Global input polling for overlay bindings.
//!
//! The overlay needs to know whether a given mouse button or key is held
//! **right now**, while another application (the game) has focus. That rules
//! out ordinary window events: the overlay's own window is click-through and
//! never receives them. So this module asks the OS directly, by polling, no
//! low-level hook, no event loop of its own. The engine already ticks ~30 fps,
//! which is plenty to catch a button press and far cheaper than installing a
//! system-wide hook (which on Windows means every process' input crosses our
//! callback, and antivirus software treats that as keylogger behaviour).
//!
//! Buttons are numbered the way the **browser** numbers them
//! (`MouseEvent.button`), because the binding is captured in the desktop app's
//! webview: 0 left, 1 middle, 2 right, 3 back, 4 forward. Translating once,
//! here, is better than having the UI guess the platform's convention.
//!
//! Keys are named by `KeyboardEvent.code` (`"KeyQ"`, `"F5"`, `"Space"`), for
//! the same reason: it's what the capture UI receives, it's layout-independent
//! (`KeyQ` is the physical key, regardless of QWERTY/AZERTY), and it maps
//! cleanly onto both a Windows virtual-key and an X11 keysym.

use serde::{Deserialize, Serialize};

/// What activates a binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Binding {
    /// Mouse button, numbered as `MouseEvent.button` (0 left … 4 forward).
    Mouse { button: u8 },
    /// Keyboard key, named as `KeyboardEvent.code`.
    Key { code: String },
}

impl Binding {
    /// Short human label for logs. The UI renders its own localized name.
    pub fn label(&self) -> String {
        match self {
            Binding::Mouse { button } => match button {
                0 => "Mouse L".into(),
                1 => "Mouse M".into(),
                2 => "Mouse R".into(),
                3 => "Mouse 4".into(),
                4 => "Mouse 5".into(),
                n => format!("Mouse {n}"),
            },
            Binding::Key { code } => code.clone(),
        }
    }
}

/// Is this binding held down right now?
///
/// Returns `false` on any platform or build without input support, which makes
/// a bound scope simply never activate rather than misbehave, the caller
/// treats "no binding" and "can't read the binding" differently (see
/// [`crate::scope`]).
pub fn is_down(binding: &Binding) -> bool {
    imp::is_down(binding)
}

/// Whether this build/platform can actually read global input. The scope uses
/// it to fall back to "always visible" instead of "never visible" where we
/// cannot poll, a lens that never appears would look like a bug, while one
/// that ignores its binding is merely the old behaviour.
pub fn available() -> bool {
    imp::AVAILABLE
}

// ───────────────────────── Windows ─────────────────────────

#[cfg(all(windows, feature = "runtime"))]
mod imp {
    use super::Binding;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

    pub const AVAILABLE: bool = true;

    /// `MouseEvent.button` → virtual-key code.
    ///
    /// Note the middle/right swap: the DOM numbers them 1 = middle, 2 = right,
    /// while Windows' VK constants go left, right, (cancel), middle.
    fn mouse_vk(button: u8) -> Option<i32> {
        Some(match button {
            0 => 0x01, // VK_LBUTTON
            1 => 0x04, // VK_MBUTTON
            2 => 0x02, // VK_RBUTTON
            3 => 0x05, // VK_XBUTTON1, the "back" thumb button
            4 => 0x06, // VK_XBUTTON2, the "forward" thumb button
            _ => return None,
        })
    }

    pub fn is_down(binding: &Binding) -> bool {
        let vk = match binding {
            Binding::Mouse { button } => match mouse_vk(*button) {
                Some(v) => v,
                None => return false,
            },
            Binding::Key { code } => match super::code_to_vk(code) {
                Some(v) => v,
                None => return false,
            },
        };
        // The high bit is "currently down"; the low bit is "pressed since the
        // last call", which we deliberately ignore, this is a level query, and
        // consuming the edge here would race with anything else polling.
        unsafe { (GetAsyncKeyState(vk) as u16) & 0x8000 != 0 }
    }
}

// ───────────────────────── X11 ─────────────────────────

#[cfg(all(target_os = "linux", feature = "runtime"))]
mod imp {
    use super::Binding;
    use std::cell::RefCell;

    use x11rb::connection::Connection;
    use x11rb::protocol::xinput::ConnectionExt as _;
    use x11rb::protocol::xproto::ConnectionExt as _;
    use x11rb::rust_connection::RustConnection;

    pub const AVAILABLE: bool = true;

    // Una conexión por hilo, abierta con pereza y reutilizada. El sondeo no
    // abre ventanas ni captura nada, así que no puede interferir con la
    // conexión propia del overlay.
    thread_local! {
        static CONN: RefCell<Option<(RustConnection, u32)>> = const { RefCell::new(None) };
    }

    fn with_conn<T>(f: impl FnOnce(&RustConnection, u32) -> Option<T>) -> Option<T> {
        CONN.with(|c| {
            let mut slot = c.borrow_mut();
            if slot.is_none() {
                let (conn, screen) = x11rb::connect(None).ok()?;
                let root = conn.setup().roots.get(screen)?.root;
                *slot = Some((conn, root));
            }
            let (conn, root) = slot.as_ref()?;
            f(conn, *root)
        })
    }

    /// `MouseEvent.button` → X11 physical button number.
    ///
    /// X numbers the wheel as buttons 4/5, so the thumb buttons land on 8/9,
    /// which is exactly why the core protocol can't report them: its
    /// `KeyButMask` only has Button1..Button5. XInput2's `XIQueryPointer`
    /// returns a full bitmask instead, so the side buttons work here.
    fn mouse_x_button(button: u8) -> Option<u16> {
        Some(match button {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 8,
            4 => 9,
            _ => return None,
        })
    }

    fn mouse_down(button: u8) -> bool {
        let Some(n) = mouse_x_button(button) else {
            return false;
        };
        with_conn(|conn, root| {
            // Device 0 = XIAllMasterDevices; the reply merges every master
            // pointer, which is what "is the user holding it" means here.
            let reply = conn
                .xinput_xi_query_pointer(root, 0u16)
                .ok()?
                .reply()
                .ok()?;
            let word = (n / 32) as usize;
            let bit = n % 32;
            Some(reply.buttons.get(word).is_some_and(|w| w & (1 << bit) != 0))
        })
        .unwrap_or(false)
    }

    fn key_down(code: &str) -> bool {
        let Some(keysym) = super::code_to_keysym(code) else {
            return false;
        };
        with_conn(|conn, _root| {
            let setup = conn.setup();
            let min = setup.min_keycode;
            let count = setup.max_keycode - min + 1;
            let map = conn.get_keyboard_mapping(min, count).ok()?.reply().ok()?;
            let per = map.keysyms_per_keycode as usize;
            if per == 0 {
                return None;
            }
            // Find the physical keycode carrying this keysym in any shift level.
            let idx = map.keysyms.chunks(per).position(|k| k.contains(&keysym))?;
            let keycode = min as usize + idx;

            // `query_keymap` is a 32-byte bitmap indexed by keycode.
            let keys = conn.query_keymap().ok()?.reply().ok()?.keys;
            let byte = keycode / 8;
            let bit = keycode % 8;
            Some(keys.get(byte).is_some_and(|b| b & (1 << bit) != 0))
        })
        .unwrap_or(false)
    }

    pub fn is_down(binding: &Binding) -> bool {
        match binding {
            Binding::Mouse { button } => mouse_down(*button),
            Binding::Key { code } => key_down(code),
        }
    }
}

// ───────────────────────── everything else ─────────────────────────

// macOS and the feature-less test build: no polling. `AVAILABLE = false` makes
// the scope ignore its binding and stay visible, which degrades to the
// behaviour that existed before bindings were a thing.
#[cfg(not(any(
    all(windows, feature = "runtime"),
    all(target_os = "linux", feature = "runtime")
)))]
mod imp {
    use super::Binding;

    pub const AVAILABLE: bool = false;

    pub fn is_down(_binding: &Binding) -> bool {
        false
    }
}

// ───────────────────────── key name tables ─────────────────────────

/// `KeyboardEvent.code` → Windows virtual-key code.
///
/// Deliberately a subset: the keys someone would actually bind to a lens.
/// Anything unlisted fails the binding rather than guessing wrong.
#[cfg(any(all(windows, feature = "runtime"), test))]
fn code_to_vk(code: &str) -> Option<i32> {
    // Letters and digits are contiguous in both namespaces.
    if let Some(c) = code.strip_prefix("Key") {
        let b = c.as_bytes();
        if b.len() == 1 && b[0].is_ascii_uppercase() {
            return Some(b[0] as i32); // VK_A..VK_Z == 'A'..'Z'
        }
    }
    if let Some(d) = code.strip_prefix("Digit") {
        let b = d.as_bytes();
        if b.len() == 1 && b[0].is_ascii_digit() {
            return Some(b[0] as i32); // VK_0..VK_9 == '0'..'9'
        }
    }
    if let Some(n) = code.strip_prefix('F') {
        if let Ok(n) = n.parse::<u8>() {
            if (1..=24).contains(&n) {
                return Some(0x70 + n as i32 - 1); // VK_F1 = 0x70
            }
        }
    }
    Some(match code {
        "Space" => 0x20,
        "Tab" => 0x09,
        "CapsLock" => 0x14,
        "ShiftLeft" => 0xA0,
        "ShiftRight" => 0xA1,
        "ControlLeft" => 0xA2,
        "ControlRight" => 0xA3,
        "AltLeft" => 0xA4,
        "AltRight" => 0xA5,
        "Backquote" => 0xC0,
        "Insert" => 0x2D,
        "Home" => 0x24,
        "PageUp" => 0x21,
        "PageDown" => 0x22,
        "End" => 0x23,
        "Delete" => 0x2E,
        _ => return None,
    })
}

/// `KeyboardEvent.code` → X11 keysym. Same subset as [`code_to_vk`].
#[cfg(any(all(target_os = "linux", feature = "runtime"), test))]
fn code_to_keysym(code: &str) -> Option<u32> {
    if let Some(c) = code.strip_prefix("Key") {
        let b = c.as_bytes();
        if b.len() == 1 && b[0].is_ascii_uppercase() {
            // Lowercase keysym: that's the unshifted level the keymap carries.
            return Some(b[0].to_ascii_lowercase() as u32);
        }
    }
    if let Some(d) = code.strip_prefix("Digit") {
        let b = d.as_bytes();
        if b.len() == 1 && b[0].is_ascii_digit() {
            return Some(b[0] as u32);
        }
    }
    if let Some(n) = code.strip_prefix('F') {
        if let Ok(n) = n.parse::<u8>() {
            if (1..=24).contains(&n) {
                return Some(0xFFBE + n as u32 - 1); // XK_F1
            }
        }
    }
    Some(match code {
        "Space" => 0x0020,
        "Tab" => 0xFF09,
        "CapsLock" => 0xFFE5,
        "ShiftLeft" => 0xFFE1,
        "ShiftRight" => 0xFFE2,
        "ControlLeft" => 0xFFE3,
        "ControlRight" => 0xFFE4,
        "AltLeft" => 0xFFE9,
        "AltRight" => 0xFFEA,
        "Backquote" => 0x0060,
        "Insert" => 0xFF63,
        "Home" => 0xFF50,
        "PageUp" => 0xFF55,
        "PageDown" => 0xFF56,
        "End" => 0xFF57,
        "Delete" => 0xFFFF,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_binding_round_trips_through_json() {
        let b = Binding::Mouse { button: 3 };
        let s = serde_json::to_string(&b).unwrap();
        assert_eq!(s, r#"{"type":"mouse","button":3}"#);
        assert_eq!(serde_json::from_str::<Binding>(&s).unwrap(), b);
    }

    #[test]
    fn key_binding_round_trips_through_json() {
        let b = Binding::Key {
            code: "KeyQ".into(),
        };
        let s = serde_json::to_string(&b).unwrap();
        assert_eq!(s, r#"{"type":"key","code":"KeyQ"}"#);
        assert_eq!(serde_json::from_str::<Binding>(&s).unwrap(), b);
    }

    #[test]
    fn letters_digits_and_function_keys_map_on_both_platforms() {
        assert_eq!(code_to_vk("KeyQ"), Some(b'Q' as i32));
        assert_eq!(code_to_vk("Digit7"), Some(b'7' as i32));
        assert_eq!(code_to_vk("F5"), Some(0x74));
        assert_eq!(code_to_keysym("KeyQ"), Some(b'q' as u32));
        assert_eq!(code_to_keysym("Digit7"), Some(b'7' as u32));
        assert_eq!(code_to_keysym("F5"), Some(0xFFC2));
    }

    #[test]
    fn unknown_key_names_are_rejected_not_guessed() {
        assert_eq!(code_to_vk("Fnord"), None);
        assert_eq!(code_to_keysym("Fnord"), None);
        assert_eq!(code_to_vk("F99"), None);
    }

    #[test]
    fn labels_name_the_thumb_buttons() {
        assert_eq!(Binding::Mouse { button: 3 }.label(), "Mouse 4");
        assert_eq!(Binding::Mouse { button: 4 }.label(), "Mouse 5");
    }
}
