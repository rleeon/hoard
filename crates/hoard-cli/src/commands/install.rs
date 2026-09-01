//! `hoard install`: leaves this machine with all the Hoard it should have, at one
//! single version.
//!
//! It is the second half of the terminal installer. The script puts the core
//! (`hoardd` plus `hoard`) in place and calls in here, which is where the decision
//! lives about whether this machine also wants the app and by which route. That
//! the decision is in Rust rather than in the `.sh` is not tidiness: `hoard
//! upgrade` and the app's updater use the same function, and having it written
//! three times in three languages is having it written wrong.
//!
//! It is also the command someone runs by hand when something was left half done,
//! no network during install, a cancelled `pkexec`, without going back to the web.

use anyhow::{Context, Result};

use hoard_agent::install::{self, Component, Delivery, Manifest, Probe};

/// Which components the user wants, when they want a say.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// Whatever the machine decides, which is the usual case.
    Detect,
    /// Core only, even with a graphical environment. For a NAS with a desktop
    /// installed, or for somebody who simply does not want the app.
    Headless,
    /// The app too, even if nothing is detected. For installing over SSH on a
    /// machine that will later be used with a screen.
    Desktop,
}

pub async fn run(want: Want, version: Option<String>) -> Result<()> {
    let probe = Probe::read();
    // The manifest beats detection *if it already exists*: reinstalling over SSH
    // cannot conclude "there is no screen here" and leave a machine that has one
    // without the app. Detection is for the first time.
    let existing = Manifest::load()?;
    let mut manifest = match (&existing, want) {
        (Some(m), Want::Detect) => m.clone(),
        _ => Manifest::planned(hoard_agent::update::current(), &probe),
    };
    match want {
        Want::Headless => {
            manifest.components.retain(|c| *c != Component::Desktop);
            manifest.delivery = None;
        }
        Want::Desktop => {
            manifest.add(Component::Desktop);
            if manifest.delivery.is_none() {
                manifest.delivery = Some(install::resolve_delivery(&probe));
            }
        }
        Want::Detect => {}
    }

    let target = match &version {
        Some(v) => v.trim_start_matches('v').to_string(),
        None => hoard_agent::update::current().to_string(),
    };
    manifest.version = target.clone();

    println!(
        "hoard {target} — components: {}",
        manifest
            .components
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // ---- core
    // It is already on disk (the script put it there, or it is us). What is
    // missing is for it to run: installing the service is what turns "there are
    // binaries" into "this syncs on its own".
    match hoardd::autostart::install().await {
        Ok(installed) => println!("  core:    service ready ({})", installed.manager),
        Err(err) => {
            // Not fatal: sync still starts when a client opens. What is lost is
            // starting at boot, and that gets said plainly.
            eprintln!("  core:    the service won't start at login — {err:#}");
        }
    }
    manifest.core_dir = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.to_path_buf()));

    // ---- app
    if manifest.has(Component::Desktop) {
        let delivery = manifest
            .delivery
            .unwrap_or_else(|| install::resolve_delivery(&probe));
        manifest.delivery = Some(delivery);
        match install_desktop(delivery, &target).await {
            Ok(path) => {
                println!(
                    "  desktop: installed ({}) → {}",
                    delivery.as_str(),
                    path.display()
                );
                manifest.desktop_path = Some(path);
            }
            Err(err) => {
                // A core installed and working is a valid outcome; the app can be
                // retried. Recording it as installed when it is not would be
                // worse than not having it, because the next `upgrade` would
                // believe it was updating it.
                manifest.components.retain(|c| *c != Component::Desktop);
                manifest.delivery = None;
                manifest.save().ok();
                eprintln!("  desktop: not installed — {err:#}");
                anyhow::bail!(
                    "the core is installed and running; the desktop app is not. \
                     Re-run `hoard install` to retry just that part."
                );
            }
        }
    }

    // Who owns the core, with everything resolved. Computed the same way as in
    // `install::observe`: the same directory *and* a route that really packages.
    // The AppImage shares a folder with the core without containing it, and taking
    // it for packaged would leave the core out of every later update on exactly
    // the SteamOS route.
    manifest.core_from_bundle = match (
        &manifest.core_dir,
        &manifest.desktop_path,
        manifest.delivery,
    ) {
        (Some(core), Some(desktop), Some(d)) => {
            d != Delivery::AppImage && desktop.parent() == Some(core.as_path())
        }
        _ => false,
    };

    manifest.save().context("writing the install manifest")?;
    Ok(())
}

/// Downloads the file this route needs, verifies its signature and applies it.
async fn install_desktop(delivery: Delivery, target: &str) -> Result<std::path::PathBuf> {
    if !delivery.is_ours() {
        anyhow::bail!(
            "the desktop app here is managed by your package manager — update it with that"
        );
    }
    // We always ask for `target`'s release, never "the latest". With `latest` this
    // broke on its own: a bare `hoard install`, the repair path this module
    // documents, resolved the newest published version, compared it against this
    // binary's version and aborted the moment a new release had shipped. What has
    // to be installed is the one matching the core already here; moving to a new
    // version is `hoard upgrade`'s job.
    let (released, assets) = install::fetch::release_assets(Some(target)).await?;
    if released != target {
        // GitHub served a different one: installing blindly would break the one
        // guarantee all of this gives, that the pieces move together.
        anyhow::bail!("asked for {target} but the release is {released}");
    }
    let asset = install::fetch::asset_for(delivery, &assets).with_context(|| {
        format!(
            "release {released} publishes no {} package",
            delivery.as_str()
        )
    })?;
    let dir = hoard_agent::config::CliConfig::cache_dir()?.join("downloads");
    println!("  desktop: downloading {}…", asset.name);
    let file = install::fetch::download_verified(asset, &assets, &dir).await?;
    let noninteractive = std::env::var_os("HOARD_NONINTERACTIVE").is_some();
    let installed = install::fetch::apply_desktop(delivery, &file, noninteractive).await?;
    // The installer is no longer needed and weighs what a bundle weighs.
    let _ = std::fs::remove_file(&file);
    Ok(installed)
}
