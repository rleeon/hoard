//! `hoard-screen`, overlay process entrypoint.
//!
//! Two modes:
//!
//! - `--snapshot <file.ppm> [--scene <file.json>] [--width W --height H]`
//!   Compose a single frame and write it as a PPM (transparent areas flattened
//!   onto a checkerboard so the crop/scale geometry is visible). This needs no
//!   display, GPU or capture backend, so it runs anywhere, including CI, and
//!   is how the full IPC→source→compositor pipeline is validated end to end.
//!
//! - (default) `run`, read newline-JSON [`Message`]s from stdin, maintain the
//!   scene, and tick the engine. On-screen presentation is the per-platform
//!   window layer (winit + GPU surface / native overlay), wired behind the
//!   `runtime` feature; without it this mode is a headless layout driver.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use hoard_screen::engine::Engine;
use hoard_screen::ipc::{parse_line, Message};
use hoard_screen::scene::{Crop, Panel, Rect, ScaleMode, Scene, SourceRef};

/// Opt the process into per-monitor DPI awareness (v2) *before* we enumerate
/// monitors or place any window. Without this the process is virtualized when
/// monitors run at different scale factors (e.g. 150% primary + 100% secondary):
/// `EnumDisplayMonitors`/`GetMonitorInfoW` report scaled coordinates and
/// `SetWindowPos` lands the panel offset/wrong-sized on the secondary screen.
/// Must run first in `main` so it also applies to the `--list-monitors` probe,
/// keeping the editor's coordinates in the same physical space as the runtime.
#[cfg(all(target_os = "windows", feature = "runtime"))]
fn set_dpi_aware() {
    use windows::Win32::UI::HiDpi::{
        SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
    };
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

#[cfg(not(all(target_os = "windows", feature = "runtime")))]
fn set_dpi_aware() {}

fn main() {
    set_dpi_aware();
    let args: Vec<String> = std::env::args().collect();

    // `--list-windows`: print the capturable windows as JSON and exit. The
    // desktop app runs this to populate the editor's picker, then forwards the
    // JSON to the Pro UI verbatim.
    if args.iter().any(|a| a == "--list-windows") {
        list_windows();
        return;
    }

    // `--list-monitors`: print the physical monitors as JSON and exit. The
    // desktop app runs this to populate the editor's per-panel screen picker.
    if args.iter().any(|a| a == "--list-monitors") {
        let mons = hoard_screen::monitors::list_monitors();
        let json = serde_json::to_string(&mons).unwrap_or_else(|_| "[]".into());
        println!("{json}");
        return;
    }

    let mut flags: HashMap<String, String> = HashMap::new();
    let mut i = 1;
    let mut snapshot: Option<String> = None;
    while i < args.len() {
        match args[i].as_str() {
            "--snapshot" => {
                snapshot = args.get(i + 1).cloned();
                i += 2;
            }
            f if f.starts_with("--") => {
                flags.insert(
                    f.trim_start_matches("--").to_string(),
                    args.get(i + 1).cloned().unwrap_or_default(),
                );
                i += 2;
            }
            _ => i += 1,
        }
    }

    // With the on-screen runtime built in, default mode opens the overlay
    // window; `--snapshot` still works for headless frame dumps.
    #[cfg(any(feature = "runtime", feature = "wayland"))]
    if snapshot.is_none() {
        hoard_screen::slog::init("runtime");
        let engine = Engine::new();
        hoard_screen::slog::line(
            "info",
            &format!("capture backend = {}", engine.backend_name()),
        );
        if let Err(e) = hoard_screen::runtime::run(engine) {
            eprintln!("hoard-screen: runtime exited: {e}");
            std::process::exit(1);
        }
        return;
    }

    if let Some(path) = snapshot {
        let w: u32 = flags
            .get("width")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1280);
        let h: u32 = flags
            .get("height")
            .and_then(|v| v.parse().ok())
            .unwrap_or(720);
        let scene = match flags.get("scene") {
            Some(p) => {
                let txt = std::fs::read_to_string(p).expect("read scene json");
                serde_json::from_str(&txt).expect("parse scene json")
            }
            None => demo_scene(w, h),
        };
        run_snapshot(&path, scene, w, h);
        return;
    }

    run_headless();
}

/// A representative layout: a test-pattern "video" panel (cropped to a centre
/// region, Fill so it fills its box) plus a captured-app placeholder panel.
fn demo_scene(w: u32, h: u32) -> Scene {
    let (fw, fh) = (w as f64, h as f64);
    Scene {
        panels: vec![
            Panel {
                id: "video".into(),
                source: SourceRef::Test,
                rect: Rect::new(fw * 0.05, fh * 0.08, fw * 0.5, fh * 0.5),
                crop: Crop {
                    top: 0.15,
                    right: 0.1,
                    bottom: 0.15,
                    left: 0.1,
                },
                scale: ScaleMode::Fill,
                z: 0,
                monitor: Default::default(),
                passthrough: true,
                compat: false,
                passthrough_radius: 90.0,
            },
            Panel {
                id: "app".into(),
                source: SourceRef::Window { id: "demo".into() },
                rect: Rect::new(fw * 0.6, fh * 0.3, fw * 0.35, fh * 0.45),
                crop: Crop::NONE,
                scale: ScaleMode::Fit,
                z: 1,
                monitor: Default::default(),
                passthrough: true,
                compat: false,
                passthrough_radius: 90.0,
            },
        ],
    }
}

fn run_snapshot(path: &str, scene: Scene, w: u32, h: u32) {
    let mut e = Engine::new();
    eprintln!("hoard-screen: capture backend = {}", e.backend_name());
    e.set_scene(scene);
    e.tick();
    let mut buf = vec![0u8; (w * h * 4) as usize];
    e.render(&mut buf, w, h);
    write_ppm(path, &buf, w, h).expect("write ppm");
    eprintln!("hoard-screen: wrote {path} ({w}x{h})");
}

/// Flatten RGBA onto a checkerboard and write binary PPM (P6).
fn write_ppm(path: &str, rgba: &[u8], w: u32, h: u32) -> std::io::Result<()> {
    let mut out = Vec::with_capacity((w * h * 3 + 64) as usize);
    out.extend_from_slice(format!("P6\n{w} {h}\n255\n").as_bytes());
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let a = rgba[i + 3] as u32;
            let bg = if ((x / 16) + (y / 16)) % 2 == 0 {
                30
            } else {
                60
            };
            for c in 0..3 {
                let v = (rgba[i + c] as u32 * a + bg * (255 - a)) / 255;
                out.push(v as u8);
            }
        }
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(&out)
}

/// Enumerate capturable windows and print them as a JSON array on stdout. Runs
/// the host OS capture backend (X11 / Wayland portal picked at runtime). On a
/// backend that can't pre-enumerate (or with no display) prints `[]` and a
/// diagnostic on stderr, the desktop app treats a non-array as "no windows".
fn list_windows() {
    let backend = hoard_screen::capture::backend();
    match backend.list_windows() {
        Ok(windows) => {
            let json = serde_json::to_string(&windows).unwrap_or_else(|_| "[]".into());
            println!("{json}");
        }
        Err(e) => {
            eprintln!("hoard-screen: list-windows failed ({e})");
            println!("[]");
        }
    }
}

/// Headless IPC loop: maintain the scene from stdin. Real on-screen
/// presentation is the `runtime` feature's job.
fn run_headless() {
    let mut e = Engine::new();
    eprintln!(
        "hoard-screen {} (capture backend: {}): headless layout driver; build with --features runtime for the window.",
        hoard_screen::version(),
        e.backend_name()
    );
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        match parse_line(&line) {
            Ok(Some(Message::SetScene { scene })) => {
                e.set_scene(scene);
                e.tick();
                eprintln!(
                    "hoard-screen: scene set, {} panel(s)",
                    e.scene().panels.len()
                );
            }
            Ok(Some(Message::SetEditor { editor })) => eprintln!("hoard-screen: editor={editor}"),
            Ok(Some(Message::GetScene)) => {
                if let Ok(s) = serde_json::to_string(e.scene()) {
                    println!("{{\"type\":\"scene\",\"scene\":{s}}}");
                }
            }
            Ok(Some(Message::Quit)) => break,
            Ok(None) => {}
            Err(err) => eprintln!("hoard-screen: bad message: {err}"),
        }
    }
}
