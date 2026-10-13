//! Download before deciding, apply all at once.
//!
//! [`super::auto`] decides *what* is needed; this does it. Two operations, and
//! the split is not cosmetic:
//!
//! - [`stage`] downloads and verifies a version's files into a directory of their
//!   own. It touches nothing installed, so it can run with a game open, with the
//!   app closed, and fail halfway with no consequences.
//! - [`apply`] puts them in place. That is the short part, a few renames, and the
//!   only one that needs the moment to be right.
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

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::fetch;
use super::{layout, Component, Delivery, Manifest};

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
    /// The core tarball. Always: the core is the one piece every install has.
    pub core: PathBuf,
    /// The app. `None` on a machine with no app.
    pub desktop: Option<PathBuf>,
    /// Hoard Setup, on Windows: the copy "Installed apps" runs to uninstall is
    /// refreshed with every update, so it never removes a newer Hoard with an
    /// older idea of where things are.
    pub setup: Option<PathBuf>,
}

/// Which app route this install updates, if it has an app.
///
/// An install from before the layout (the NSIS folder, the MSI) is not updated
/// in place: it is repaired, by Hoard Setup, which moves it. Staging an update
/// for it would download files this machine has nowhere to put. A `.deb` or
/// `.rpm` the person kept gets the core and nothing for the app.
fn wanted(manifest: &Manifest) -> Result<Option<Delivery>> {
    if !manifest.has(Component::Desktop) {
        return Ok(None);
    }
    let d = manifest
        .delivery
        .context("the manifest says there's an app here but not how it got here")?;
    if !d.is_ours() {
        bail!(
            "this install is managed by your package manager ({})",
            d.as_str()
        );
    }
    // A package the person kept: the core moves on, the app is theirs.
    if d.is_package() {
        return Ok(None);
    }
    if d.is_legacy() {
        bail!(
            "this install predates the current layout ({}); it needs a repair, not an update",
            d.as_str()
        );
    }
    Ok(Some(d))
}

fn setup_wanted() -> bool {
    cfg!(windows)
}

/// What is already downloaded for `version`, if it is complete.
///
/// Complete matters: a download cut in half leaves the core tarball and not the
/// app, and taking it for good would apply half an update. A missing file counts
/// as nothing being there, and it gets downloaded again.
pub fn already_staged(version: &str, manifest: &Manifest) -> Option<Staged> {
    let delivery = wanted(manifest).ok()?;
    let dir = dir(version).ok()?;
    let present = |name: String| {
        let path = dir.join(name);
        path.is_file().then_some(path)
    };
    let core = present(fetch::core_asset_name(version)?)?;
    let desktop = match delivery {
        Some(d) => Some(present(fetch::app_asset_name(d, version)?)?),
        None => None,
    };
    let setup = if setup_wanted() {
        Some(present(fetch::setup_asset_name()?)?)
    } else {
        None
    };
    Some(Staged {
        version: version.trim_start_matches('v').to_string(),
        core,
        desktop,
        setup,
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
    let delivery = wanted(manifest)?;
    let dir = dir(&version)?;

    let core_name = fetch::core_asset_name(&version).with_context(|| {
        format!(
            "no core build for {}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })?;
    let core = fetch::download_named(&version, &core_name, &dir).await?;
    let desktop = match delivery {
        Some(d) => Some(fetch::download_app(d, &version, &dir).await?),
        None => None,
    };
    let setup = match fetch::setup_asset_name().filter(|_| setup_wanted()) {
        Some(name) => Some(fetch::download_named(&version, &name, &dir).await?),
        None => None,
    };
    Ok(Staged {
        version,
        core,
        desktop,
        setup,
    })
}

/// Puts what was downloaded in place and records the new version in the manifest.
///
/// The core first, because the other pieces depend on it. Nothing here asks for
/// privileges or for anybody's attention: every piece goes into the user's own
/// directories, which is what lets the service do this with the app closed.
pub async fn apply(staged: &Staged, manifest: &mut Manifest) -> Result<()> {
    // From here to the end the binaries on disk are in motion, and a client that
    // starts a service off them would run half an update. The guard is what tells
    // those clients to sit still.
    let _swap = super::Swap::begin();

    let core_dir = layout::core_dir()?;
    let written = fetch::apply_core(&staged.core, &core_dir).await?;
    tracing::info!(
        version = %staged.version,
        files = ?written,
        "update: core replaced"
    );
    manifest.core_dir = Some(core_dir);

    if let Some(file) = &staged.desktop {
        let delivery = manifest
            .delivery
            .context("the manifest says there's an app here but not how it got here")?;
        let path = fetch::apply_desktop(delivery, file).await?;
        tracing::info!(
            version = %staged.version,
            delivery = delivery.as_str(),
            path = %path.display(),
            "update: desktop replaced"
        );
        #[cfg(windows)]
        if let Err(err) = super::windows::register(&path, &staged.version, false).await {
            tracing::warn!(error = %format!("{err:#}"), "update: couldn't refresh the Windows entries");
        }
        manifest.desktop_path = Some(path);
    }

    #[cfg(windows)]
    if let Some(setup) = &staged.setup {
        if let Err(err) = super::windows::keep_setup_copy(setup) {
            tracing::warn!(error = %format!("{err:#}"), "update: couldn't refresh the uninstaller");
        }
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

    fn manifest(desktop: Option<Delivery>) -> Manifest {
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
        }
    }

    #[test]
    fn a_headless_box_only_wants_the_core() {
        assert_eq!(wanted(&manifest(None)).unwrap(), None);
    }

    #[test]
    fn a_desktop_box_wants_its_app_too() {
        assert_eq!(
            wanted(&manifest(Some(Delivery::AppImage))).unwrap(),
            Some(Delivery::AppImage)
        );
    }

    /// The old Windows installers have nothing to stage; they have a repair to
    /// run.
    #[test]
    fn an_old_windows_install_is_repaired_not_updated() {
        for d in [Delivery::Nsis, Delivery::Msi] {
            let err = wanted(&manifest(Some(d))).unwrap_err();
            assert!(format!("{err:#}").contains("repair"), "{d:?}: {err:#}");
        }
    }

    /// A `.deb` or `.rpm` the person kept: the update brings the core and
    /// leaves their package alone.
    #[test]
    fn a_kept_package_stages_the_core_alone() {
        for d in [Delivery::Deb, Delivery::Rpm] {
            assert_eq!(wanted(&manifest(Some(d))).unwrap(), None, "{d:?}");
        }
    }

    #[test]
    fn a_managed_install_has_nothing_to_stage() {
        assert!(wanted(&manifest(Some(Delivery::Managed))).is_err());
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
}
