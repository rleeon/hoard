//! IPC with the desktop app. The Tauri process owns the editor and the asset
//! cache; this overlay process is launched by it and fed layout updates.
//!
//! Wire format: newline-delimited JSON, one [`Message`] per line, on stdin.
//! Trivial, dependency-free, and easy to drive from Tauri's `Command` sidecar
//! API (or a unix socket later). The overlay replies (nav state, picked window
//! ids) the same way on stdout.

use serde::{Deserialize, Serialize};

use crate::scene::Scene;

/// Desktop -> overlay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// Replace the whole layout.
    SetScene { scene: Scene },
    /// Force a mode (e.g. the desktop's own "edit overlay" button); the global
    /// Ctrl+O hotkey toggles it locally too.
    SetEditor { editor: bool },
    /// Ask the overlay to emit its current mode + scene on stdout right now.
    /// The desktop sends this on (re)mount to resync its editor with what the
    /// overlay is actually showing, its own copy of the scene can go stale
    /// (webview reload, missed event), which left panels on screen that the
    /// editor no longer listed and so could never be moved or removed.
    GetScene,
    /// Ask the overlay to shut down cleanly.
    Quit,
}

/// Parse one newline-stripped line into a [`Message`]. Blank lines yield `None`
/// so the reader can skip keepalives without erroring.
pub fn parse_line(line: &str) -> Result<Option<Message>, serde_json::Error> {
    let t = line.trim();
    if t.is_empty() {
        return Ok(None);
    }
    serde_json::from_str(t).map(Some)
}

/// Serialise a message as a single line (no trailing newline).
pub fn to_line<T: Serialize>(msg: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Crop, Panel, Rect, ScaleMode, SourceRef};

    #[test]
    fn blank_lines_are_skipped() {
        assert_eq!(parse_line("   ").unwrap(), None);
    }

    #[test]
    fn round_trips_set_scene() {
        let scene = Scene {
            panels: vec![Panel {
                id: "v1".into(),
                source: SourceRef::Window { id: "0xabc".into() },
                rect: Rect::new(10.0, 10.0, 640.0, 360.0),
                crop: Crop {
                    top: 0.1,
                    right: 0.0,
                    bottom: 0.1,
                    left: 0.0,
                },
                scale: ScaleMode::Fill,
                z: 2,
                monitor: Default::default(),
                passthrough: true,
                compat: false,
                passthrough_radius: 90.0,
            }],
        };
        let msg = Message::SetScene { scene };
        let line = to_line(&msg).unwrap();
        assert_eq!(parse_line(&line).unwrap(), Some(msg));
    }

    #[test]
    fn defaults_fill_in_optional_panel_fields() {
        // crop/scale/z omitted -> defaults.
        let line = r#"{"type":"set_scene","scene":{"panels":[
            {"id":"n","source":{"kind":"note","text":"hi"},
             "rect":{"x":0,"y":0,"w":100,"h":50}}]}}"#;
        let Message::SetScene { scene } = parse_line(line).unwrap().unwrap() else {
            panic!("expected set_scene");
        };
        assert_eq!(scene.panels[0].scale, ScaleMode::Fill);
        assert_eq!(scene.panels[0].z, 0);
        assert_eq!(scene.panels[0].crop, Crop::NONE);
    }

    #[test]
    fn crosshair_parses_from_bare_kind_with_defaults() {
        // The editor may send just the kind; every spec field has a default.
        let line = r#"{"type":"set_scene","scene":{"panels":[
            {"id":"ch","source":{"kind":"crosshair"},
             "rect":{"x":936,"y":516,"w":48,"h":48}}]}}"#;
        let Message::SetScene { scene } = parse_line(line).unwrap().unwrap() else {
            panic!("expected set_scene");
        };
        let SourceRef::Crosshair(spec) = &scene.panels[0].source else {
            panic!("expected crosshair source");
        };
        assert_eq!(*spec, crate::crosshair::CrosshairSpec::default());
        // And the full spec round-trips.
        let msg = Message::SetScene { scene };
        let line = to_line(&msg).unwrap();
        assert_eq!(parse_line(&line).unwrap(), Some(msg));
    }

    #[test]
    fn unknown_source_kind_degrades_instead_of_dropping_the_scene() {
        // A newer editor may send kinds this build doesn't know; the line must
        // still parse (that one panel becomes SourceRef::Unknown) so the rest
        // of the scene survives.
        let line = r#"{"type":"set_scene","scene":{"panels":[
            {"id":"f","source":{"kind":"hologram","spin":9},
             "rect":{"x":0,"y":0,"w":10,"h":10}},
            {"id":"w","source":{"kind":"window","id":"0xabc"},
             "rect":{"x":0,"y":0,"w":10,"h":10}}]}}"#;
        let Message::SetScene { scene } = parse_line(line).unwrap().unwrap() else {
            panic!("expected set_scene");
        };
        assert_eq!(scene.panels[0].source, SourceRef::Unknown);
        assert_eq!(
            scene.panels[1].source,
            SourceRef::Window { id: "0xabc".into() }
        );
    }

    #[test]
    fn parses_quit() {
        assert_eq!(
            parse_line(r#"{"type":"quit"}"#).unwrap(),
            Some(Message::Quit)
        );
    }

    #[test]
    fn parses_get_scene_and_scope_defaults() {
        assert_eq!(
            parse_line(r#"{"type":"get_scene"}"#).unwrap(),
            Some(Message::GetScene)
        );
        let line = r#"{"type":"set_scene","scene":{"panels":[
            {"id":"z","source":{"kind":"scope"},
             "rect":{"x":0,"y":0,"w":300,"h":300}}]}}"#;
        let Message::SetScene { scene } = parse_line(line).unwrap().unwrap() else {
            panic!("expected set_scene");
        };
        let SourceRef::Scope(spec) = &scene.panels[0].source else {
            panic!("expected scope source");
        };
        assert_eq!(*spec, crate::scope::ScopeSpec::default());
    }
}
