//! **Hoard Setup**: the window, and the thread that does the work behind it.
//!
//! There is no webview here on purpose. This is the first thing that runs on a
//! machine where nothing is installed, so it cannot depend on WebView2 or
//! WebKitGTK being present: those are among the things it is here to install.
//! Slint compiles the UI into this binary and the software renderer draws it
//! without asking the GPU for anything, which is what makes it start inside a
//! VM, over RDP, and on a handheld in whatever mode it happens to be in.
//!
//! The split is the usual one and it is not negotiable: Slint's event loop owns
//! the main thread, so the install runs on a Tokio runtime on another one and
//! reports back through [`slint::Weak::upgrade_in_event_loop`].

#![cfg_attr(windows, windows_subsystem = "windows")]

mod steps;

use std::sync::Arc;

slint::include_modules!();

/// The eight the app itself speaks, in the order the picker lists them.
///
/// `(tag, badge, name)`: the tag selects the bundled `.po`, the badge is what
/// the button shows, and the name is the language written in itself, since somebody
/// looking for Deutsch is not helped by the word "German" in a language they
/// don't read.
const LANGUAGES: [(&str, &str, &str); 8] = [
    ("en", "EN", "English"),
    ("es", "ES", "Español"),
    ("fr", "FR", "Français"),
    ("de", "DE", "Deutsch"),
    ("pt", "PT", "Português"),
    ("it", "IT", "Italiano"),
    ("ja", "JA", "日本語"),
    ("zh", "ZH", "中文"),
];

/// Screen indices, matching `installer.slint`.
const WELCOME: i32 = 0;
const ASK: i32 = 1;
const DOING: i32 = 2;
const DONE: i32 = 3;
const FAILED: i32 = 4;
const EXISTING: i32 = 5;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First thing, while this is still single-threaded: `set_var` is not sound
    // once the Tokio runtime and the event loop have threads of their own.
    if let Ok(daemon) = steps::daemon_binary() {
        std::env::set_var(hoardd::client::DAEMON_BIN_ENV, daemon);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    let flag = |name: &str| args.iter().any(|a| a.eq_ignore_ascii_case(name));
    // `/S` is what a Windows installer answers to, and this one should answer to
    // it too: it is how an unattended deployment drives any of them, and how
    // this one gets tested on a machine nobody is looking at.
    if flag("--silent") || flag("/S") || flag("--uninstall") {
        return silent(flag("--uninstall"), !flag("--no-desktop"));
    }

    // Before the window exists, so the first frame is already in the right
    // language rather than flashing English and correcting itself.
    let start_lang = system_language();
    let _ = slint::select_bundled_translation(LANGUAGES[start_lang].0);

    let ui = Installer::new()?;
    ui.set_version(env!("CARGO_PKG_VERSION").into());
    ui.set_language(start_lang as i32);
    ui.set_language_codes(slint::ModelRc::new(slint::VecModel::from(
        LANGUAGES
            .iter()
            .map(|(_, badge, _)| slint::SharedString::from(*badge))
            .collect::<Vec<_>>(),
    )));
    ui.set_language_names(slint::ModelRc::new(slint::VecModel::from(
        LANGUAGES
            .iter()
            .map(|(_, _, name)| slint::SharedString::from(*name))
            .collect::<Vec<_>>(),
    )));
    ui.on_pick_language(|index| {
        if let Some((tag, _, _)) = LANGUAGES.get(index as usize) {
            // Every `@tr` in the tree re-evaluates on its own from here.
            let _ = slint::select_bundled_translation(tag);
        }
    });

    // Ask the disk before showing anything: on a machine that already has
    // Hoard, "Continue" leads somewhere different, and finding that out after
    // the user has committed to installing would be finding it out too late.
    let found = steps::detect();
    if let Some(existing) = &found {
        ui.set_installed(true);
        ui.set_installed_version(existing.version.clone().unwrap_or_default().into());
        ui.set_installed_has_desktop(existing.has_desktop());
    }
    let found = Arc::new(found);

    // Slint owns the main thread from `run()` on, so every await lives here.
    let rt = Arc::new(tokio::runtime::Runtime::new()?);

    // Replace the compiled-in label with the release that will actually be
    // installed, as soon as GitHub answers.
    {
        let weak = ui.as_weak();
        rt.spawn(async move {
            if let Ok(version) = steps::latest_version().await {
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    // Which decides whether the button says Update or Reinstall.
                    ui.set_up_to_date(ui.get_installed_version() == version.as_str());
                    ui.set_version(version.into());
                });
            }
        });
    }

    ui.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    ui.on_go_ask({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                // An install this machine already has is not a question about
                // which pieces it wants; it is a question about what to do next.
                ui.set_screen(if ui.get_installed() { EXISTING } else { ASK });
            }
        }
    });

    ui.on_go_back({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_screen(WELCOME);
            }
        }
    });

    ui.on_open_license(|| {
        open_url("https://www.gnu.org/licenses/agpl-3.0.html");
    });

    ui.on_start_install({
        let weak = ui.as_weak();
        let rt = rt.clone();
        let found = found.clone();
        move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            // On an update, what to install is what is already there, so the
            // toggle was never shown, so its default must not decide.
            let want_desktop = match found.as_ref() {
                Some(existing) if ui.get_installed() => existing.has_desktop(),
                _ => ui.get_want_desktop(),
            };
            ui.set_message(Default::default());
            ui.set_kept_path(Default::default());
            ui.set_screen(DOING);

            let weak = weak.clone();
            rt.spawn(async move {
                let result = steps::run(want_desktop).await;
                let _ = weak.upgrade_in_event_loop(move |ui| match result {
                    Ok(outcome) => {
                        if let Some(path) = outcome.launch {
                            ui.set_launch_path(path.to_string_lossy().to_string().into());
                            ui.set_can_launch(true);
                        }
                        ui.set_screen(DONE);
                    }
                    Err(err) => {
                        // The agent's messages are written for a person, so
                        // they go through unedited. `{err:#}` keeps the chain:
                        // "installing the desktop app: dpkg exited with 1" says
                        // where it broke, which "1" alone does not.
                        ui.set_message(format!("{err:#}").into());
                        ui.set_screen(FAILED);
                    }
                });
            });
        }
    });

    ui.on_start_uninstall({
        let weak = ui.as_weak();
        let rt = rt.clone();
        let found = found.clone();
        move || {
            let (Some(ui), Some(existing)) = (weak.upgrade(), found.as_ref().clone()) else {
                return;
            };
            ui.set_message(Default::default());
            ui.set_screen(DOING);

            let weak = weak.clone();
            rt.spawn(async move {
                let result = steps::uninstall(&existing).await;
                let _ = weak.upgrade_in_event_loop(move |ui| match result {
                    Ok(kept) => {
                        ui.set_can_launch(false);
                        // The path only. The sentence around it is in the UI,
                        // where a translation can reach it.
                        if let Some(path) = kept.first() {
                            ui.set_kept_path(path.display().to_string().into());
                        }
                        ui.set_screen(DONE);
                    }
                    Err(err) => {
                        ui.set_message(format!("{err:#}").into());
                        ui.set_screen(FAILED);
                    }
                });
            });
        }
    });

    ui.on_launch({
        let weak = ui.as_weak();
        move || {
            if let Some(ui) = weak.upgrade() {
                let path = ui.get_launch_path().to_string();
                if !path.is_empty() {
                    spawn_detached(&path);
                }
                let _ = slint::quit_event_loop();
            }
        }
    });

    // A frameless window has no bar for the system to drag, so we move it.
    ui.on_drag({
        let weak = ui.as_weak();
        move |dx, dy| {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let window = ui.window();
            let at = window.position().to_logical(window.scale_factor());
            window.set_position(slint::LogicalPosition::new(at.x + dx, at.y + dy));
        }
    });

    present(ui.as_weak());
    ui.run()?;
    Ok(())
}

/// Puts the window in the middle of the screen and in front, once.
///
/// Once is the whole point. An installer that opens behind the browser that
/// downloaded it looks broken, and one that stays pinned above everything is
/// worse, since you can't put it aside to read anything while it runs. So it is
/// raised on the way in and dropped back to an ordinary window a moment later,
/// after which clicking any other window puts that one in front, as it should.
///
/// It waits on `winit_window()` rather than reaching for the window straight
/// away: Slint creates it when the event loop gets going, which is *after*
/// `show()` returns, so anything eager finds nothing there and silently does
/// nothing at all.
fn present(weak: slint::Weak<Installer>) {
    use slint::winit_030::{winit, WinitWindowAccessor};

    let _ = slint::spawn_local(async move {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let Ok(window) = ui.window().winit_window().await else {
            return;
        };

        // Centre on the monitor it actually landed on, not on the primary one:
        // with two screens, "the primary monitor" is the wrong answer half the
        // time.
        if let Some(monitor) = window
            .current_monitor()
            .or_else(|| window.primary_monitor())
        {
            let screen = monitor.size();
            let origin = monitor.position();
            let size = window.outer_size();
            window.set_outer_position(winit::dpi::PhysicalPosition::new(
                origin.x + (screen.width as i32 - size.width as i32) / 2,
                origin.y + (screen.height as i32 - size.height as i32) / 2,
            ));
        }

        window.set_window_level(winit::window::WindowLevel::AlwaysOnTop);
        window.focus_window();

        // Back to an ordinary window. Long enough that the compositor has
        // actually raised it, short enough that nobody could have clicked past
        // it yet.
        slint::Timer::single_shot(std::time::Duration::from_millis(400), move || {
            window.set_window_level(winit::window::WindowLevel::Normal);
        });
    });
}

/// The whole thing with no window: install, or take it off, and say what
/// happened in a log beside the temp directory.
///
/// A GUI binary on Windows has no console to print to, so the outcome goes to a
/// file and to the exit code. That is also what makes it usable from a script,
/// which is the reason installers have had a silent switch since forever.
fn silent(uninstall: bool, want_desktop: bool) -> Result<(), Box<dyn std::error::Error>> {
    let log = std::env::temp_dir().join("hoard-setup.log");
    let rt = tokio::runtime::Runtime::new()?;

    let outcome: Result<String, String> = rt.block_on(async {
        if uninstall {
            let found = steps::detect().ok_or("nothing to uninstall: no Hoard on this machine")?;
            let kept = steps::uninstall(&found)
                .await
                .map_err(|e| format!("{e:#}"))?;
            Ok(format!(
                "uninstalled. kept: {}",
                kept.iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        } else {
            let done = steps::run(want_desktop)
                .await
                .map_err(|e| format!("{e:#}"))?;
            Ok(match done.launch {
                Some(path) => format!("installed. app at {}", path.display()),
                None => "installed. core only".to_string(),
            })
        }
    });

    let (line, code) = match &outcome {
        Ok(msg) => (format!("OK: {msg}\n"), 0),
        Err(err) => (format!("FAILED: {err}\n"), 1),
    };
    let _ = std::fs::write(&log, &line);
    print!("{line}");
    std::process::exit(code);
}

/// Which of [`LANGUAGES`] this machine asks for, falling back to English.
///
/// Matched on the language part alone: `pt-BR` and `pt-PT` both get `pt`,
/// which is the honest resolution when only one Portuguese is bundled.
fn system_language() -> usize {
    let Some(locale) = sys_locale::get_locale() else {
        return 0;
    };
    let tag = locale
        .split(['-', '_'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    LANGUAGES
        .iter()
        .position(|(code, _, _)| *code == tag)
        .unwrap_or(0)
}

/// Starts the freshly installed app and lets go of it: the installer is about
/// to exit, and a child that dies with its parent would take the app with it.
fn spawn_detached(path: &str) {
    let mut command = std::process::Command::new(path);
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Out of our process group, so it survives us.
        // SAFETY: `setsid` takes no arguments and only touches the child.
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }
    let _ = command.spawn();
}

/// Opens a URL in whatever the system considers a browser.
fn open_url(url: &str) {
    #[cfg(target_os = "linux")]
    let (program, args) = ("xdg-open", vec![url]);
    #[cfg(target_os = "macos")]
    let (program, args) = ("open", vec![url]);
    #[cfg(target_os = "windows")]
    let (program, args) = ("cmd", vec!["/C", "start", "", url]);

    let _ = std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
