//! **The install itself.** Everything the window doesn't do, which is nearly
//! everything.
//!
//! Not one line here decides anything: what this machine wants, which file
//! matches it, how it is checked, what an older release left behind and in which
//! order it all comes off live in `hoardd::setup`, the same sequence
//! `hoard install` and the service's own repairs run. A second copy of that
//! reasoning behind a pretty window is how you end up with an installer that
//! puts a different Hoard on the machine than the terminal does.

use std::path::PathBuf;

use anyhow::Result;
use hoard_agent::install::{self, fetch};
use hoardd::setup::{Outcome, Plan, Source, Want};

/// Which release this installer is about to fetch.
///
/// The welcome screen starts out labelled with the installer's *own* version,
/// which is right the day it ships and wrong for every copy that sits in a
/// downloads folder for a month. This asks, so the label says what will
/// actually land. A failure is silent on purpose: the network is needed for
/// the install itself, and there is a screen that reports that properly.
pub async fn latest_version() -> Result<String> {
    fetch::latest_version().await
}

/// Install, update or repair: one call, at one version.
///
/// `want` is the only thing the window gets to decide, and only on a first
/// install; on a machine that has Hoard it is [`Want::Keep`].
pub async fn run(
    want: Want,
    version: Option<String>,
    from: Option<PathBuf>,
    app: Option<PathBuf>,
    interactive: bool,
) -> Result<Outcome> {
    let (source, app_file) = match from {
        Some(dir) => {
            let (app, _) = hoardd::setup::from_dir(&dir);
            (Source::Local(dir), app)
        }
        None => (Source::Download, None),
    };
    let app_file = app.or(app_file);
    hoardd::setup::run(Plan {
        version,
        want,
        source,
        interactive,
        // This very program is what "Installed apps" will run to uninstall.
        setup_binary: std::env::current_exe().ok(),
        app_file,
    })
    .await
}

/// What this machine already has, if anything.
pub async fn detect() -> Option<install::Installed> {
    install::detect().await
}

/// Take Hoard off this machine. Returns where the user's own things still are:
/// **nothing under those paths is touched**.
pub async fn uninstall(interactive: bool) -> Result<Vec<PathBuf>> {
    hoardd::setup::uninstall(interactive).await
}
