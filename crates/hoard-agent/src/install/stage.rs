//! Download before deciding, apply all at once.
//!
//! [`super::auto`] decides *what* is needed; this does it. Two operations, and
//! the split is not cosmetic:
//!
//! - [`stage`] downloads and verifies a version's files into a directory of their
//!   own. It touches nothing installed, so it can run with a game open, with the
//!   app closed, and fail halfway with no consequences.
//! - [`apply`] puts them in place. That is the short part, renaming two binaries
//!   or running an installer, and the only one that needs the moment to be right.
//!
//! Splitting it that way is what makes "update when you open it" something that
//! can be promised: by the time you open it there is no download left, only a
//! `rename`. It is what Steam and Discord do, for the same reason.
//!
//! ## One version, one directory
//!
//! What has been downloaded lives in `<cache>/staged/<version>/`. With the version
//! in the path, a service restart halfway through a download leaves no files from
//! two releases mixed in one folder, and [`sweep`] can delete the old without
//! looking inside. Every file is checked against the release key before it is
//! written, so nothing unsigned ever sits in `staged/`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::fetch;
use super::{Component, Delivery, Manifest};

/// Where a version's downloads are kept.
pub fn dir(version: &str) -> Result<PathBuf> {
    Ok(crate::config::CliConfig::cache_dir()?
        .join("staged")
        .join(version.trim_start_matches('v')))
}

/// A version's files, on disk and already verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    pub version: String,
    /// The core tarball. `None` when the core is not ours (it travels inside the
    /// app bundle, whose installer relieves it).
    pub core: Option<PathBuf>,
    /// The app installer. `None` on a machine with no app.
    pub desktop: Option<PathBuf>,
}

impl Staged {
    /// Is there anything to apply? An empty `Staged` means this install has none
    /// of our pieces, and applying it would bump the manifest's version number
    /// without a single binary having been touched.
    pub fn is_empty(&self) -> bool {
        self.core.is_none() && self.desktop.is_none()
    }
}

/// Which files this install needs in order to move to `version`.
fn wanted(version: &str, manifest: &Manifest) -> Result<(bool, Option<Delivery>)> {
    // We relieve the core ourselves unless the bundle brings it.
    let core = !manifest.core_from_bundle;
    let desktop = if manifest.has(Component::Desktop) {
        let d = manifest
            .delivery
            .context("the manifest says there's an app here but not how it got here")?;
        if !d.is_ours() {
            bail!(
                "this install is managed by your package manager ({})",
                d.as_str()
            );
        }
        Some(d)
    } else {
        None
    };
    if !core && desktop.is_none() {
        bail!("nothing here is ours to update for {version}");
    }
    Ok((core, desktop))
}

/// What is already downloaded for `version`, if it is complete.
///
/// Complete matters: a download cut in half leaves the core tarball and not the
/// app installer, and taking it for good would apply half an update. A missing
/// file counts as nothing being there, and it gets downloaded again.
pub fn already_staged(version: &str, manifest: &Manifest) -> Option<Staged> {
    let (want_core, want_desktop) = wanted(version, manifest).ok()?;
    let dir = dir(version).ok()?;

    let core = if want_core {
        let name = fetch::core_asset_name(version)?;
        let path = dir.join(name);
        if !path.is_file() {
            return None;
        }
        Some(path)
    } else {
        None
    };

    let desktop = match want_desktop {
        Some(_) => {
            // The bundle's exact name is unknown without listing the release, so
            // the one file that is not the core tarball is accepted.
            let core_name = core
                .as_ref()
                .and_then(|p| p.file_name().map(|s| s.to_owned()));
            let mut others = std::fs::read_dir(&dir)
                .ok()?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .filter(|p| p.file_name().map(|n| Some(n.to_owned()) != core_name) == Some(true));
            let found = others.next()?;
            if others.next().is_some() {
                // Two candidates: there is no telling which. Re-downloading is
                // cheap and guessing wrong runs an installer that wasn't it.
                return None;
            }
            Some(found)
        }
        None => None,
    };

    Some(Staged {
        version: version.trim_start_matches('v').to_string(),
        core,
        desktop,
    })
}

/// Downloads and verifies everything needed to move to `version`.
///
/// It applies nothing. Idempotent: whatever was already downloaded is not
/// downloaded again.
pub async fn stage(version: &str, manifest: &Manifest) -> Result<Staged> {
    let version = version.trim_start_matches('v').to_string();
    if let Some(done) = already_staged(&version, manifest) {
        return Ok(done);
    }

    let (want_core, want_desktop) = wanted(&version, manifest)?;
    let dir = dir(&version)?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;

    let (released, assets) = fetch::release_assets(Some(&version)).await?;
    if released != version {
        // The same guard as `hoard install`: downloading "something else" breaks
        // the one guarantee all of this gives, that the pieces move together.
        bail!("asked GitHub for {version} but the release is {released}");
    }

    let core = if want_core {
        let asset = fetch::core_asset_for(&version, &assets).with_context(|| {
            format!(
                "release {version} publishes no core tarball for {}-{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })?;
        Some(fetch::download_verified(asset, &assets, &dir).await?)
    } else {
        None
    };

    let desktop = match want_desktop {
        Some(delivery) => {
            let asset = fetch::asset_for(delivery, &assets).with_context(|| {
                format!(
                    "release {version} publishes no {} package",
                    delivery.as_str()
                )
            })?;
            Some(fetch::download_verified(asset, &assets, &dir).await?)
        }
        None => None,
    };

    Ok(Staged {
        version,
        core,
        desktop,
    })
}

/// Puts what was downloaded in place and records the new version in the manifest.
///
/// The order is [`super`]'s: the core first, because the other pieces depend on
/// it, and because it is the one that applies without being able to fail on a
/// human cancellation. If the app then fails, on a `pkexec` the user cancels, the
/// manifest does not move up a version: it keeps pointing at the old one and the
/// next cycle retries the whole thing. Recording a version that only reached half
/// the pieces is manufacturing the silent mismatch this exists to prevent.
///
/// `noninteractive` shuts off any route that could stop to ask. The service always
/// sets it: it has no window to draw a dialog in, so a `pkexec` launched from
/// there would wait forever.
pub async fn apply(staged: &Staged, manifest: &mut Manifest, noninteractive: bool) -> Result<()> {
    if staged.is_empty() {
        bail!("there is nothing staged for {}", staged.version);
    }

    // From here to the end of this function the binaries on disk are in motion,
    // and a client that starts a service off them would either run half an update
    // or, on Windows, hold open the very file the installer is trying to write.
    // The guard is what tells those clients to sit still.
    let _swap = super::Swap::begin();

    if let Some(tarball) = &staged.core {
        let dir = manifest
            .core_dir
            .clone()
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|e| e.parent().map(Path::to_path_buf))
            })
            .context("don't know where the core lives on this machine")?;
        let written = fetch::apply_core(tarball, &dir).await?;
        tracing::info!(
            version = %staged.version,
            files = ?written,
            "update: core replaced"
        );
        manifest.core_dir = Some(dir);
    }

    if let Some(installer) = &staged.desktop {
        let delivery = manifest
            .delivery
            .context("the manifest says there's an app here but not how it got here")?;
        let path = fetch::apply_desktop(delivery, installer, noninteractive).await?;
        tracing::info!(
            version = %staged.version,
            delivery = delivery.as_str(),
            path = %path.display(),
            "update: desktop replaced"
        );
        manifest.desktop_path = Some(path);
    }

    manifest.version.clone_from(&staged.version);
    manifest.save().context("writing the install manifest")?;
    sweep(&staged.version);
    Ok(())
}

/// Deletes what was downloaded for other versions. Called after applying, when
/// nothing from before is needed; a bundle is tens of megabytes and letting them
/// pile up in the cache is the failure nobody sees until the disk fills.
pub fn sweep(keep: &str) {
    let Ok(cache) = crate::config::CliConfig::cache_dir() else {
        return;
    };
    let root = cache.join("staged");
    let Ok(entries) = std::fs::read_dir(&root) else {
        return;
    };
    let keep = keep.trim_start_matches('v');
    for entry in entries.flatten() {
        if entry.file_name() == keep {
            continue;
        }
        let _ = std::fs::remove_dir_all(entry.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(desktop: Option<Delivery>, core_from_bundle: bool) -> Manifest {
        let mut components = vec![Component::Core];
        if desktop.is_some() {
            components.push(Component::Desktop);
        }
        Manifest {
            version: "1.0.0".into(),
            components,
            delivery: desktop,
            core_dir: None,
            desktop_path: None,
            core_from_bundle,
        }
    }

    #[test]
    fn a_headless_box_only_wants_the_core() {
        let (core, desktop) = wanted("1.1.0", &manifest(None, false)).unwrap();
        assert!(core);
        assert_eq!(desktop, None);
    }

    #[test]
    fn a_desktop_box_wants_both() {
        let (core, desktop) = wanted("1.1.0", &manifest(Some(Delivery::AppImage), false)).unwrap();
        assert!(core);
        assert_eq!(desktop, Some(Delivery::AppImage));
    }

    #[test]
    fn a_bundled_core_rides_the_app_installer() {
        let (core, desktop) = wanted("1.1.0", &manifest(Some(Delivery::Deb), true)).unwrap();
        assert!(!core, "the .deb brings the core with it");
        assert_eq!(desktop, Some(Delivery::Deb));
    }

    #[test]
    fn a_managed_install_has_nothing_to_stage() {
        assert!(wanted("1.1.0", &manifest(Some(Delivery::Managed), true)).is_err());
    }

    #[test]
    fn a_bundled_core_with_no_app_is_a_contradiction() {
        // `core_from_bundle` with no `Desktop` leaves none of our pieces: there
        // is nothing to download, and saying so here avoids creating an empty
        // directory and "applying" it to bump the manifest version for nothing.
        assert!(wanted("1.1.0", &manifest(None, true)).is_err());
    }

    #[test]
    fn the_staging_dir_is_per_version_and_drops_the_tag_prefix() {
        let a = dir("1.1.0").unwrap();
        let b = dir("v1.1.0").unwrap();
        assert_eq!(a, b);
        assert!(
            a.ends_with("staged/1.1.0") || a.ends_with(r"staged\1.1.0"),
            "{a:?}"
        );
        assert_ne!(a, dir("1.1.1").unwrap());
    }

    #[tokio::test]
    async fn applying_nothing_is_an_error_not_a_version_bump() {
        let mut m = manifest(None, false);
        let empty = Staged {
            version: "1.1.0".into(),
            core: None,
            desktop: None,
        };
        assert!(apply(&empty, &mut m, true).await.is_err());
        assert_eq!(m.version, "1.0.0");
    }
}
