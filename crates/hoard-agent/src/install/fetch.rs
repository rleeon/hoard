//! Downloading and applying a component: which release file this machine needs,
//! how it is checked to be ours, and how it gets installed.
//!
//! It lives in the agent rather than in a frontend because both update paths use
//! it, `hoard upgrade` from the terminal and the app's button, and they are the
//! same operation. Duplicating it here would mean that one day the app installs a
//! `.deb` where the terminal installs an AppImage, and the user ends up with two
//! different Hoards on the same machine.
//!
//! None of this opens a window or needs one: picking an asset, verifying the
//! signature and calling `dpkg` or `rpm` is ordinary business logic. What stays in
//! the desktop is asking the user and drawing the progress.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::Delivery;

/// Repo de las releases. Mismo que resuelven `install.sh` / `install.ps1`.
const REPO: &str = "rleeon/hoard";

/// The minisign public key CI signs everything published with (ADR 0017; the
/// signing job is isolated from the building one so no third-party dependency ever
/// sees the private key).
///
/// A binary without a valid signature is not installed. It is the only real
/// defence between "I download an executable off the internet" and "I run it with
/// privileges": GitHub's TLS says the file arrived intact, not that we published
/// it.
pub const MINISIGN_PUBKEY: &str = "RWSeOL1nHXZI9oa+WOdrc6yVasLPeBurvGWnERo4tN9F+YIQn7ipx3eO";

/// Un fichero publicado en la release.
#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    #[serde(default)]
    assets: Vec<Asset>,
    #[serde(default)]
    tag_name: String,
}

/// A release's files. `None` as the version means the latest published one.
pub async fn release_assets(version: Option<&str>) -> Result<(String, Vec<Asset>)> {
    let url = match version {
        Some(v) => format!(
            "https://api.github.com/repos/{REPO}/releases/tags/v{}",
            v.trim_start_matches('v')
        ),
        None => format!("https://api.github.com/repos/{REPO}/releases/latest"),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("hoard/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .with_context(|| format!("asking GitHub for {url}"))?;
    if !resp.status().is_success() {
        bail!(
            "GitHub answered {} for {url}; the release may not be published yet",
            resp.status()
        );
    }
    let rel: Release = resp.json().await.context("parsing the release")?;
    Ok((rel.tag_name.trim_start_matches('v').to_string(), rel.assets))
}

/// The file this delivery route needs.
///
/// It is looked up by route rather than by "let's see what is there": the route
/// was already decided by [`super::resolve_delivery`] looking at the machine
/// (immutable root, available package manager, whether we can elevate), and
/// guessing again here by suffix would mean the policy is written twice and
/// disagrees with itself.
pub fn asset_for(delivery: Delivery, assets: &[Asset]) -> Option<&Asset> {
    let suffixes: &[&str] = match delivery {
        Delivery::Deb => &[".deb"],
        Delivery::Rpm => &[".rpm"],
        Delivery::AppImage => &[".AppImage"],
        // NSIS before MSI, and the order matters now that `hoardd` outlives the
        // window: the installer has to overwrite a `hoardd.exe` the service holds
        // open, and only the NSIS bundle carries the hook that stops it first
        // (`installer-hooks.nsh`). Through the MSI that hook never runs and the
        // update dies against the locked file.
        Delivery::Nsis => &["-setup.exe", ".exe", ".msi"],
        Delivery::Dmg => &[".dmg"],
        // Maintained by a third party: there is no file of ours to download.
        Delivery::Managed => return None,
    };
    suffixes.iter().find_map(|suffix| {
        let mut matching = assets
            .iter()
            .filter(|a| {
                a.name.ends_with(suffix) && !a.name.ends_with(".minisig") && !is_our_installer(a)
            })
            .peekable();
        // Sin candidatos, nada que decidir.
        matching.peek()?;
        let candidates: Vec<&Asset> = matching.collect();
        pick_for_arch(&candidates)
    })
}

/// Is this asset the graphical installer rather than a package of the app?
///
/// Every release carries both, and they collide by construction: the installer
/// is a `.exe` on Windows and would be an `.AppImage` or a `.dmg` if it were
/// packaged the way each system expects, the very suffixes [`asset_for`] uses
/// to recognise the app. [`pick_for_arch`] can't separate them either, since
/// both carry the same architecture token, so the tie would be broken by
/// whatever order GitHub happens to list the files in.
///
/// Getting it wrong is quiet and bad: the in-app updater would download the
/// installer, run it as an update, and the user would get a window asking them
/// to install what they already have.
///
/// Matched on the name because that is the only thing a release asset has.
/// CI publishes ours as `HoardSetup-<arch>[.ext]`; the check is the contract,
/// so renaming there means changing this too.
fn is_our_installer(asset: &Asset) -> bool {
    asset.name.to_ascii_lowercase().starts_with("hoardsetup")
}

/// The tokens the bundles use to name our architecture.
fn arch_tokens() -> &'static [&'static str] {
    match std::env::consts::ARCH {
        "x86_64" => &["x86_64", "amd64", "x64"],
        "aarch64" => &["aarch64", "arm64"],
        _ => &[],
    }
}

/// Every architecture token we know how to recognise, ours or not.
const KNOWN_ARCH_TOKENS: &[&str] = &["x86_64", "amd64", "x64", "aarch64", "arm64"];

/// Out of several files of the same format, the one for this architecture.
///
/// Today each release publishes a single bundle per system, so "take the first"
/// is right by luck. It stops being right the moment a second one ships: it would
/// pick by whichever order GitHub happens to list the files in, and an amd64
/// `.deb` on an ARM machine is not a loud failure but a `dpkg` complaining about
/// something that does not look related. And the core tarball already ships for
/// ARM, so the machine that can land here exists today.
///
/// If no candidate carries an architecture token, the release does not
/// distinguish and the first one will do. If they do carry one but none is ours,
/// `None` comes back on purpose: "there is no package for your architecture" is a
/// useful answer, and installing somebody else's is not.
fn pick_for_arch<'a>(candidates: &[&'a Asset]) -> Option<&'a Asset> {
    let ours = arch_tokens();
    if let Some(hit) = candidates
        .iter()
        .find(|a| ours.iter().any(|t| contains_token(&a.name, t)))
    {
        return Some(hit);
    }
    let any_tagged = candidates
        .iter()
        .any(|a| KNOWN_ARCH_TOKENS.iter().any(|t| contains_token(&a.name, t)));
    if any_tagged {
        return None;
    }
    candidates.first().copied()
}

/// Does `token` appear as a piece of the name rather than inside another word?
///
/// The boundaries are checked instead of splitting on separators, and that is not
/// a matter of style: the most important token, `x86_64`, has a `_` inside it, so
/// splitting on `_` destroys it before it can be looked for and no file named the
/// usual way would ever match. A boundary is anything non-alphanumeric, or the
/// start or end of the name.
fn contains_token(name: &str, token: &str) -> bool {
    let hay = name.to_ascii_lowercase();
    let needle = token.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(rel) = hay[from..].find(&needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Downloads an asset and its `.minisig`, verifies the signature and leaves the
/// file on disk. Returns where it landed.
///
/// It fails closed: with no `.minisig` published, or a signature that does not
/// match, nothing applicable gets written. An installer runs with privileges, and
/// "it is probably fine" is not a policy.
pub async fn download_verified(
    asset: &Asset,
    assets: &[Asset],
    dest_dir: &Path,
) -> Result<PathBuf> {
    let sig_name = format!("{}.minisig", asset.name);
    let sig = assets
        .iter()
        .find(|a| a.name == sig_name)
        .with_context(|| {
            format!(
                "{} has no published signature ({sig_name}). Refusing to install an \
                 unverified installer.",
                asset.name
            )
        })?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .user_agent(concat!("hoard/", env!("CARGO_PKG_VERSION")))
        .build()?;
    let bytes = client
        .get(&asset.url)
        .send()
        .await
        .with_context(|| format!("downloading {}", asset.name))?
        .error_for_status()?
        .bytes()
        .await?;
    let sig_text = client
        .get(&sig.url)
        .send()
        .await
        .with_context(|| format!("downloading {sig_name}"))?
        .error_for_status()?
        .text()
        .await?;

    verify(&bytes, &sig_text).with_context(|| format!("verifying {}", asset.name))?;

    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("creating {}", dest_dir.display()))?;
    let path = dest_dir.join(&asset.name);
    std::fs::write(&path, &bytes).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Comprueba `bytes` contra la firma minisign `sig_text` con [`MINISIGN_PUBKEY`].
pub fn verify(bytes: &[u8], sig_text: &str) -> Result<()> {
    use minisign_verify::{PublicKey, Signature};

    let pubkey = PublicKey::from_base64(MINISIGN_PUBKEY)
        .map_err(|e| anyhow::anyhow!("the embedded public key is unusable: {e}"))?;
    let signature = Signature::decode(sig_text)
        .map_err(|e| anyhow::anyhow!("the signature file is malformed: {e}"))?;
    pubkey
        .verify(bytes, &signature, false)
        .map_err(|e| anyhow::anyhow!("signature does not match: {e}"))?;
    Ok(())
}

/// Installs the already-verified file according to its route.
///
/// Native packages need privileges; the AppImage touches nothing outside the
/// home. `noninteractive` shuts off any route that could stop to ask: inside
/// `curl ... | sh` there is nobody to ask, and hanging is worse than not
/// installing.
pub async fn apply_desktop(
    delivery: Delivery,
    path: &Path,
    noninteractive: bool,
) -> Result<PathBuf> {
    match delivery {
        Delivery::Deb => {
            elevated(&["dpkg", "-i"], path, noninteractive).await?;
            Ok(PathBuf::from("/usr/bin/hoard-desktop"))
        }
        Delivery::Rpm => {
            elevated(&["rpm", "-U", "--force"], path, noninteractive).await?;
            Ok(PathBuf::from("/usr/bin/hoard-desktop"))
        }
        Delivery::AppImage => place_appimage(path),
        Delivery::Nsis => {
            // The installer takes care of it (and carries the hook that stops the
            // service before touching `hoardd.exe`). `/S` means silent.
            let status = tokio::process::Command::new(path)
                .arg("/S")
                .status()
                .await
                .with_context(|| format!("running {}", path.display()))?;
            if !status.success() {
                bail!("the installer exited with {status}");
            }
            // Where it *landed*, not the setup file we just ran. Returning the
            // installer's own path put a `…\Temp\Hoard_x.y.z_x64-setup.exe`
            // into `Manifest::desktop_path`, a file that gets swept minutes
            // later. Two things then break quietly: "Open Hoard" runs the
            // installer again instead of the app, and an uninstall looks for a
            // path that no longer exists, finds nothing, and reports success
            // having removed nothing.
            super::installed_desktop().with_context(|| {
                format!(
                    "{} ran and reported success, but no hoard-desktop.exe turned up where \
                     it installs",
                    path.display()
                )
            })
        }
        Delivery::Dmg => install_dmg(path).await,
        Delivery::Managed => {
            bail!("this install is managed by your package manager; nothing to do")
        }
    }
}

/// Mounts the `.dmg`, copies the `.app` into `/Applications`, unmounts.
///
/// This is the macOS half of "the installer does the same thing on all three
/// systems". It used to fail on purpose ("open it and drag Hoard to
/// Applications"), which is reasonable guidance for a person with the Finder in
/// front of them and no guidance at all for the updater, or for an install
/// window that claims to be installing.
///
/// Mounted with `-nobrowse` so a Finder window doesn't open on top of ours, and
/// unmounted whatever happens above: a volume nobody detaches sits on the
/// user's desktop until they reboot.
#[cfg(target_os = "macos")]
async fn install_dmg(path: &Path) -> Result<PathBuf> {
    use tokio::process::Command;

    let out = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-plist"])
        .arg(path)
        .output()
        .await
        .context("running `hdiutil attach`")?;
    if !out.status.success() {
        bail!(
            "`hdiutil attach` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let mount = mount_point(&String::from_utf8_lossy(&out.stdout)).with_context(|| {
        format!(
            "{} mounted but no volume came back from hdiutil",
            path.display()
        )
    })?;

    let result = copy_app_out(&mount).await;

    // Whatever happened above, the volume gets detached. `-force` because
    // Spotlight may still be indexing it and a polite unmount would fail.
    let _ = Command::new("hdiutil")
        .args(["detach", "-force"])
        .arg(&mount)
        .output()
        .await;

    result
}

/// The mount point inside the plist `hdiutil attach -plist` prints.
///
/// No plist parser: we look for the `<string>` that starts with `/Volumes/`,
/// the only field in that output shaped like this. A whole XML dependency to
/// read one path would be expensive for the same answer.
#[cfg(target_os = "macos")]
fn mount_point(plist: &str) -> Option<PathBuf> {
    plist
        .split("<string>")
        .skip(1)
        .filter_map(|rest| rest.split("</string>").next())
        .map(str::trim)
        .find(|s| s.starts_with("/Volumes/"))
        .map(PathBuf::from)
}

/// Copies the volume's single `.app` into `/Applications`, or into the home
/// directory when that folder isn't ours.
///
/// `ditto` rather than `cp -R`: it preserves extended attributes and code
/// signatures, and a bundle copied without them is a bundle Gatekeeper refuses
/// to open. The destination is removed first instead of copied over, because an
/// update that leaves stray files from the previous version inside the bundle
/// produces failures that look nothing like their cause.
#[cfg(target_os = "macos")]
async fn copy_app_out(mount: &Path) -> Result<PathBuf> {
    use tokio::process::Command;

    let app = std::fs::read_dir(mount)
        .with_context(|| format!("listing {}", mount.display()))?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .with_context(|| format!("no .app inside {}", mount.display()))?;
    let name = app
        .file_name()
        .context("the .app bundle has no name")?
        .to_owned();

    // `/Applications` is the right answer and isn't always writable (an account
    // without admin rights). `~/Applications` always is, and Launchpad looks
    // there too, so the fallback is a complete install and not a consolation
    // prize.
    let mut targets = vec![PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        targets.push(home.join("Applications"));
    }

    let mut last = String::from("no destination directory");
    for dir in targets {
        if std::fs::create_dir_all(&dir).is_err() && !dir.is_dir() {
            last = format!("{} is not writable", dir.display());
            continue;
        }
        let dest = dir.join(&name);
        if dest.exists() {
            if let Err(e) = std::fs::remove_dir_all(&dest) {
                last = format!("replacing {}: {e}", dest.display());
                continue;
            }
        }
        let out = Command::new("ditto")
            .arg(&app)
            .arg(&dest)
            .output()
            .await
            .context("running `ditto`")?;
        if out.status.success() {
            return Ok(dest);
        }
        last = format!(
            "copying to {}: {}",
            dest.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    bail!("could not install the app bundle: {last}")
}

/// Off macOS a `.dmg` is installable by nobody. We never get here:
/// [`super::resolve_delivery`] only picks it on macOS, and this exists so
/// [`apply_desktop`]'s match stays total on the other platforms.
#[cfg(not(target_os = "macos"))]
async fn install_dmg(path: &Path) -> Result<PathBuf> {
    bail!(
        "{} is a macOS disk image and this isn't macOS",
        path.display()
    )
}

/// Runs a package manager with privileges: `pkexec` first (it draws its own
/// dialog and does not depend on this terminal), `sudo -n` otherwise.
///
/// Neither route may sit waiting for a human, which is what would happen inside
/// `curl ... | sh`, where stdin is the script itself: `sudo` always carries `-n`,
/// so with no cached credential it fails instantly, and `pkexec` is only chosen
/// when there is a graphical session to draw the dialog on (which
/// [`super::can_elevate`] checks before the route ever gets here).
///
/// `noninteractive` closes the remaining gap: [`super::can_elevate`] accepts
/// `pkexec` when `$DISPLAY` is set, but `$DISPLAY` can be set with no polkit agent
/// listening (SSH with X11 forwarding is the typical case) and then `pkexec` waits
/// for a dialog nobody is going to draw. With the flag set, only root and
/// `sudo -n` count: failing with a message always beats hanging an installer.
async fn elevated(cmd: &[&str], path: &Path, noninteractive: bool) -> Result<()> {
    let mut argv: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
    argv.push(path.to_string_lossy().to_string());
    elevated_argv(&argv, noninteractive).await
}

/// The same, for a command whose last word isn't a path: `dpkg -r hoard`
/// takes a package name, not a file, and taking one away needs the same
/// privileges as putting it there.
pub(super) async fn elevated_argv(argv: &[String], noninteractive: bool) -> Result<()> {
    let argv: Vec<String> = argv.to_vec();

    let is_root = {
        #[cfg(unix)]
        {
            // SAFETY: `geteuid` no toma argumentos y no falla.
            unsafe { libc::geteuid() == 0 }
        }
        #[cfg(not(unix))]
        {
            false
        }
    };

    let (program, args): (String, Vec<String>) = if is_root {
        (argv[0].clone(), argv[1..].to_vec())
    } else if which("pkexec") && !noninteractive {
        ("pkexec".into(), argv)
    } else if which("sudo") {
        let mut a = vec!["-n".to_string()];
        a.extend(argv);
        ("sudo".into(), a)
    } else if noninteractive {
        bail!(
            "this package needs privileges and nothing here can grant them without asking \
             (run `hoard install` yourself from a terminal, or use `--headless`)"
        )
    } else {
        bail!("no way to get the privileges this package needs (no pkexec, no sudo)")
    };

    let status = tokio::process::Command::new(&program)
        .args(&args)
        .status()
        .await
        .with_context(|| format!("running `{program}`"))?;
    if !status.success() {
        bail!("`{program} {}` exited with {status}", args.join(" "));
    }
    Ok(())
}

fn which(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|d| d.join(name).is_file()))
        .unwrap_or(false)
}

/// Puts the AppImage in the home and gives it a menu entry.
///
/// No `sudo` and no package manager: it is the route that works where the other
/// two cannot (SteamOS, Bazzite, Arch). The engine does NOT go inside it, since
/// the installer put it on a stable path, which is what lets this machine's sync
/// start at boot even though the app is an AppImage.
fn place_appimage(downloaded: &Path) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("no HOME in the environment")?;
    let bin = home.join(".local").join("bin");
    std::fs::create_dir_all(&bin).with_context(|| format!("creating {}", bin.display()))?;
    let dest = bin.join("hoard-desktop");

    // It writes to a temporary and renames over, never a direct `copy`: if the app
    // is open, and updating from the app itself is the normal case, the kernel
    // refuses to write over its own executable with `ETXTBSY`. A `rename` over a
    // running binary is fine: the live process keeps its inode and the name comes
    // to point at the new one. And it is atomic into the bargain, so a failure
    // halfway does not leave a truncated AppImage where a working one was.
    let staging = bin.join(".hoard-desktop.new");
    let _ = std::fs::remove_file(&staging);
    std::fs::copy(downloaded, &staging)
        .with_context(|| format!("staging the AppImage at {}", staging.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staging, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("making {} executable", staging.display()))?;
    }
    if let Err(e) = std::fs::rename(&staging, &dest) {
        let _ = std::fs::remove_file(&staging);
        return Err(e).with_context(|| format!("installing the AppImage at {}", dest.display()));
    }
    write_desktop_entry(&home, &dest)?;
    Ok(dest)
}

/// The menu entry. Without it the AppImage exists but cannot be launched from
/// anywhere but a terminal, and in gaming mode that is not existing.
fn write_desktop_entry(home: &Path, exe: &Path) -> Result<()> {
    let dir = home.join(".local").join("share").join("applications");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let entry = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Hoard\n\
         Comment=Game save sync\n\
         Exec=\"{}\"\n\
         Icon=hoard\n\
         Terminal=false\n\
         Categories=Utility;Game;\n",
        exe.display()
    );
    let path = dir.join("dev.hoard.desktop.desktop");
    std::fs::write(&path, entry).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

// ---- the core: `hoardd` plus `hoard`, with no shell installer
//
// `hoard upgrade` used to relieve the core by piping `curl ... | sh` into an
// interpreter. That works for a person typing in a terminal and for nothing else:
// the service has no terminal, a `curl` that is not installed turns "it updates
// itself" into "it does not update and does not say so", and the script ends by
// calling `hoard install`, meaning a process we have just replaced on disk. What
// follows is the same operation written where it can be supervised: download the
// tarball, check its signature, and replace the two binaries.
//
// None of this asks for privileges when the core is where the installer leaves it
// (`~/.local/bin`, `%LOCALAPPDATA%`). If it is in `/usr/bin` then a package put it
// there, and then it is not ours: the app's installer relieves it and nothing here
// touches it (`Manifest::core_from_bundle`).

/// How a release names this machine's core tarball.
///
/// The name is fixed by CI and also resolved by `install.sh` and `install.ps1`; it
/// is written here a third time because the alternative, guessing by suffix as
/// [`asset_for`] does, cannot tell the Linux tarball from the Windows one: both
/// end in `.tar.gz`.
pub fn core_asset_name(version: &str) -> Option<String> {
    let os = match std::env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some(format!(
        "hoard-{}-{os}-{arch}.tar.gz",
        version.trim_start_matches('v')
    ))
}

/// The core tarball inside an already-listed release.
pub fn core_asset_for<'a>(version: &str, assets: &'a [Asset]) -> Option<&'a Asset> {
    let want = core_asset_name(version)?;
    assets.iter().find(|a| a.name == want)
}

/// The two binaries that make up the core. The order does not matter here; what
/// matters is that both are present before anything is touched (see
/// [`apply_core`]).
const CORE_BINARIES: [&str; 2] = ["hoard", "hoardd"];

/// Pulls `hoard` and `hoardd` out of the already-verified tarball and leaves them
/// in `dir`, replacing whatever was there.
///
/// Returns the paths written.
///
/// All or nothing: first both are extracted to files beside them, and only once
/// both are whole on disk are they renamed over. Half an update, a new `hoard`
/// talking to an old `hoardd`, is worse than none, because the handshake tolerates
/// it and the mismatch says nothing.
pub async fn apply_core(tarball: &Path, dir: &Path) -> Result<Vec<PathBuf>> {
    let bytes = tokio::fs::read(tarball)
        .await
        .with_context(|| format!("reading {}", tarball.display()))?;
    let staged = extract_core(&bytes, dir).await?;

    let mut written = Vec::new();
    for (name, temp) in staged {
        let dest = dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        replace_binary(&temp, &dest).with_context(|| format!("replacing {}", dest.display()))?;
        written.push(dest);
    }
    Ok(written)
}

/// Extracts both binaries to `<dir>/.<name>.new`. It fails when either is missing,
/// and cleans up whatever it had written before failing.
async fn extract_core(tarball: &[u8], dir: &Path) -> Result<Vec<(&'static str, PathBuf)>> {
    use async_compression::tokio::bufread::GzipDecoder;
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;

    let decoder = GzipDecoder::new(tokio::io::BufReader::new(std::io::Cursor::new(tarball)));
    let mut archive = tokio_tar::Archive::new(decoder);
    let mut entries = archive.entries().context("reading the core tarball")?;

    let mut found: Vec<(&'static str, PathBuf)> = Vec::new();
    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("reading a tarball entry")?;
        let path = entry
            .path()
            .context("a tarball entry has no path")?
            .into_owned();
        // The tarball carries a root directory (`hoard-<ver>-<platform>/`), but
        // that is not assumed: only the last component is looked at, so a change
        // in packaging does not break the update silently.
        let stem = path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.trim_end_matches(".exe"));
        let Some(name) = CORE_BINARIES.iter().find(|w| Some(**w) == stem) else {
            continue;
        };
        if found.iter().any(|(n, _)| n == name) {
            continue;
        }
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .await
            .with_context(|| format!("reading {name} out of the tarball"))?;

        let temp = dir.join(format!(".{name}.new"));
        let _ = std::fs::remove_file(&temp);
        let mut out = tokio::fs::File::create(&temp)
            .await
            .with_context(|| format!("creating {}", temp.display()))?;
        out.write_all(&buf).await?;
        out.flush().await?;
        drop(out);
        make_executable(&temp)?;
        found.push((name, temp));
    }

    if found.len() != CORE_BINARIES.len() {
        for (_, temp) in &found {
            let _ = std::fs::remove_file(temp);
        }
        let missing: Vec<&str> = CORE_BINARIES
            .iter()
            .copied()
            .filter(|w| !found.iter().any(|(n, _)| n == w))
            .collect();
        bail!(
            "the release tarball is missing {}; refusing to install half of the core",
            missing.join(" and ")
        );
    }
    Ok(found)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("chmod +x {}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// Puts `src` where `dest` was, with `dest` running.
///
/// It is the only place this can be done properly, and each system does it for a
/// different reason:
///
/// - Unix: a `rename` over a live process's executable is legal. The process keeps
///   its inode (it carries on running the old binary until it restarts) and the
///   name comes to point at the new one. Writing *inside* the file is not: the
///   kernel answers `ETXTBSY`, which is what broke a direct copy.
/// - Windows: an open `.exe` cannot be replaced, but it can be renamed. So the old
///   one is moved aside and the new one takes its name; the one moved aside is
///   deleted on the next pass, once nobody holds it open.
fn replace_binary(src: &Path, dest: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        // Sweeping the previous attempt. Best-effort: if the old process is still
        // alive it will still refuse to be deleted, and that is fine.
        let parked = dest.with_extension("old");
        let _ = std::fs::remove_file(&parked);
        if dest.exists() {
            std::fs::rename(dest, &parked).with_context(|| {
                format!(
                    "parking the running {}; is another copy still starting?",
                    dest.display()
                )
            })?;
        }
    }
    std::fs::rename(src, dest)
        .with_context(|| format!("moving {} into place as {}", src.display(), dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How the bundlers spell *this* machine's architecture, so the fixture
    /// below describes a release that actually has a file for the runner. A
    /// hardcoded `amd64` made these tests assert on x86 and panic on ARM;
    /// `pick_for_arch` is right to return None when no candidate is ours, and
    /// macOS CI runs on Apple Silicon.
    fn arch_token() -> &'static str {
        match std::env::consts::ARCH {
            "aarch64" => "arm64",
            _ => "amd64",
        }
    }

    fn assets() -> Vec<Asset> {
        let arch = arch_token();
        ["deb", "rpm", "AppImage", "dmg"]
            .iter()
            .flat_map(|ext| {
                let name = format!("Hoard_1.2.0_{arch}.{ext}");
                [
                    Asset {
                        url: format!("https://example.invalid/{name}"),
                        name: name.clone(),
                    },
                    Asset {
                        url: format!("https://example.invalid/{name}.minisig"),
                        name: format!("{name}.minisig"),
                    },
                ]
            })
            .chain([Asset {
                name: format!(
                    "Hoard_1.2.0_{}-setup.exe",
                    if arch == "arm64" { "arm64" } else { "x64" }
                ),
                url: "https://example.invalid/setup".into(),
            }])
            .collect()
    }

    #[test]
    fn each_delivery_picks_its_own_file() {
        let a = assets();
        assert!(asset_for(Delivery::Deb, &a).unwrap().name.ends_with(".deb"));
        assert!(asset_for(Delivery::Rpm, &a).unwrap().name.ends_with(".rpm"));
        assert!(asset_for(Delivery::AppImage, &a)
            .unwrap()
            .name
            .ends_with(".AppImage"));
        assert!(asset_for(Delivery::Dmg, &a).unwrap().name.ends_with(".dmg"));
        assert!(asset_for(Delivery::Nsis, &a)
            .unwrap()
            .name
            .ends_with("-setup.exe"));
    }

    /// The signature can never pass for the artefact itself: `.deb.minisig` also
    /// "ends in .deb" if you look carelessly, and running `dpkg -i` on a signature
    /// file is an absurd failure to diagnose.
    #[test]
    fn a_signature_is_never_mistaken_for_the_artifact() {
        let a = assets();
        for d in [
            Delivery::Deb,
            Delivery::Rpm,
            Delivery::AppImage,
            Delivery::Dmg,
        ] {
            assert!(!asset_for(d, &a).unwrap().name.ends_with(".minisig"));
        }
    }

    /// A third-party install has no file of ours to download. Returning `Some`
    /// here would end up overwriting from underneath whatever the distro's package
    /// manager put in place.
    #[test]
    fn a_managed_install_has_nothing_to_fetch() {
        assert!(asset_for(Delivery::Managed, &assets()).is_none());
    }

    #[test]
    fn a_release_without_our_format_reports_nothing() {
        let only_windows = vec![Asset {
            name: "Hoard_1.2.0_x64-setup.exe".into(),
            url: "https://example.invalid/setup".into(),
        }];
        assert!(asset_for(Delivery::Deb, &only_windows).is_none());
    }

    /// A release with two architectures cannot be resolved by whichever order
    /// GitHub lists the files in. Today only one per system is published, so "the
    /// first" is right by luck; the day the second ships, this is what stops an
    /// amd64 `.deb` landing on an ARM machine.
    #[test]
    fn a_two_arch_release_picks_this_machines_arch() {
        let two = vec![
            Asset {
                name: "Hoard_1.2.0_arm64.deb".into(),
                url: "u".into(),
            },
            Asset {
                name: "Hoard_1.2.0_amd64.deb".into(),
                url: "u".into(),
            },
        ];
        let want = if cfg!(target_arch = "x86_64") {
            "Hoard_1.2.0_amd64.deb"
        } else {
            "Hoard_1.2.0_arm64.deb"
        };
        assert_eq!(
            asset_for(Delivery::Deb, &two).map(|a| a.name.as_str()),
            Some(want)
        );
    }

    /// And if the release only carries another architecture, better to say there
    /// is nothing than to install the wrong package: `dpkg`'s error about a
    /// foreign architecture leads nowhere.
    #[test]
    fn a_release_for_another_arch_only_reports_nothing() {
        let other = if cfg!(target_arch = "x86_64") {
            "Hoard_1.2.0_arm64.deb"
        } else {
            "Hoard_1.2.0_amd64.deb"
        };
        let only_other = vec![Asset {
            name: other.into(),
            url: "u".into(),
        }];
        assert!(asset_for(Delivery::Deb, &only_other).is_none());
    }

    /// A single-architecture release labels nothing, and that has to keep working,
    /// since it is today's case.
    #[test]
    fn an_untagged_release_still_resolves() {
        let untagged = vec![Asset {
            name: "Hoard.deb".into(),
            url: "u".into(),
        }];
        assert_eq!(
            asset_for(Delivery::Deb, &untagged).map(|a| a.name.as_str()),
            Some("Hoard.deb")
        );
    }

    /// The token is a piece of the name, not a substring: `x64` must not match
    /// inside `x86_64` or the other way round.
    #[test]
    fn arch_tokens_match_whole_parts_only() {
        // The token with a `_` inside it is the one that broke the first version
        // of this.
        assert!(contains_token("hoard-1.2.0-linux-x86_64.tar.gz", "x86_64"));
        assert!(contains_token("Hoard_1.2.0_x64-setup.exe", "x64"));
        assert!(contains_token("Hoard_1.2.0_amd64.deb", "amd64"));
        assert!(contains_token("Hoard_1.2.0_aarch64.dmg", "aarch64"));
        // Y no cuela dentro de otra palabra.
        assert!(!contains_token("Hoard_1.2.0_x86_64.deb", "x64"));
        assert!(!contains_token("Hoard_prearm64x.deb", "arm64"));
    }

    /// It fails closed: junk does not verify against a signature. It is the
    /// assertion that separates "I downloaded a file" from "I downloaded *our*
    /// file".
    #[test]
    fn a_bogus_signature_does_not_verify() {
        assert!(verify(b"payload", "untrusted comment: nope\nnot-a-signature\n").is_err());
    }

    #[test]
    fn the_core_tarball_is_named_after_this_machine() {
        let name = core_asset_name("1.1.2").expect("this target should be supported");
        assert!(name.starts_with("hoard-1.1.2-"), "{name}");
        assert!(name.ends_with(".tar.gz"), "{name}");
        assert!(name.contains(std::env::consts::ARCH), "{name}");
        // The tag's `v` does not travel in the file name.
        assert_eq!(core_asset_name("v1.1.2"), Some(name));
    }

    /// A tarball laid out the way CI publishes it: a root directory with both
    /// binaries inside. It is built with the same pieces that read it
    /// ([`tokio_tar`] plus `async-compression`'s gzip), so the test does not depend
    /// on a packer other than the one we will actually meet.
    async fn core_tarball(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use async_compression::tokio::write::GzipEncoder;
        use tokio::io::AsyncWriteExt;

        let mut tar = tokio_tar::Builder::new(Vec::new());
        for (name, body) in entries {
            let mut header = tokio_tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, name, *body).await.unwrap();
        }
        let raw = tar.into_inner().await.unwrap();

        let mut gz = GzipEncoder::new(Vec::new());
        gz.write_all(&raw).await.unwrap();
        gz.shutdown().await.unwrap();
        gz.into_inner()
    }

    #[tokio::test]
    async fn extracting_the_core_writes_both_halves_next_to_the_old_ones() {
        let dir = tempdir();
        let tarball = core_tarball(&[
            ("hoard-9.9.9-linux-x86_64/hoard", b"new-cli"),
            ("hoard-9.9.9-linux-x86_64/hoardd", b"new-engine"),
        ])
        .await;
        let staged = extract_core(&tarball, &dir).await.unwrap();
        assert_eq!(staged.len(), 2);
        for (name, temp) in &staged {
            assert!(temp.exists(), "{name} was not written");
            assert_eq!(temp.file_name().unwrap(), format!(".{name}.new").as_str());
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn half_a_core_is_refused_and_leaves_nothing_behind() {
        let dir = tempdir();
        // The terminal only: a new `hoard` against an old `hoardd` is the silent
        // mismatch this whole module exists not to create.
        let tarball = core_tarball(&[("hoard-9.9.9-linux-x86_64/hoard", b"new-cli")]).await;
        let err = extract_core(&tarball, &dir).await.unwrap_err();
        assert!(err.to_string().contains("hoardd"), "{err}");
        assert!(
            !dir.join(".hoard.new").exists(),
            "the partial write was kept"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn applying_the_core_replaces_what_was_there() {
        let dir = tempdir();
        let exe = std::env::consts::EXE_SUFFIX;
        std::fs::write(dir.join(format!("hoard{exe}")), b"old-cli").unwrap();
        std::fs::write(dir.join(format!("hoardd{exe}")), b"old-engine").unwrap();

        let tarball = core_tarball(&[
            ("hoard-9.9.9-linux-x86_64/hoard", b"new-cli"),
            ("hoard-9.9.9-linux-x86_64/hoardd", b"new-engine"),
        ])
        .await;
        let tar_path = dir.join("core.tar.gz");
        std::fs::write(&tar_path, &tarball).unwrap();

        let written = apply_core(&tar_path, &dir).await.unwrap();
        assert_eq!(written.len(), 2);
        assert_eq!(
            std::fs::read(dir.join(format!("hoard{exe}"))).unwrap(),
            b"new-cli"
        );
        assert_eq!(
            std::fs::read(dir.join(format!("hoardd{exe}"))).unwrap(),
            b"new-engine"
        );
        // No leftovers: the temporaries were consumed by the rename.
        assert!(!dir.join(".hoard.new").exists());
        assert!(!dir.join(".hoardd.new").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hoard-core-install-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}

#[cfg(test)]
mod installer_guard_tests {
    use super::*;

    fn asset(name: &str) -> Asset {
        Asset {
            name: name.to_string(),
            url: format!("https://example.invalid/{name}"),
        }
    }

    /// The release that ships both: the updater has to reach past our own
    /// installer and find the app.
    ///
    /// The app's names carry no architecture token on purpose, so
    /// [`pick_for_arch`] falls through to "the only candidate" and the outcome
    /// depends on nothing but the filter under test. With real names the answer
    /// would change with the machine running the test: a `.dmg` is published
    /// for aarch64 only, so on an x86_64 host the right answer is `None`, and
    /// this test would be asserting the host rather than the code.
    #[test]
    fn picks_the_app_not_the_installer() {
        let assets = vec![
            asset("HoardSetup-x86_64.AppImage"),
            asset("Hoard.AppImage"),
            asset("HoardSetup-x86_64.exe"),
            asset("Hoard-setup.exe"),
            asset("HoardSetup-aarch64.dmg"),
            asset("Hoard.dmg"),
        ];

        for (delivery, want) in [
            (Delivery::AppImage, "Hoard.AppImage"),
            (Delivery::Nsis, "Hoard-setup.exe"),
            (Delivery::Dmg, "Hoard.dmg"),
        ] {
            assert_eq!(
                asset_for(delivery, &assets).map(|a| a.name.as_str()),
                Some(want),
                "{delivery:?} picked the wrong file"
            );
        }
    }

    /// And with only the installer there, the honest answer is "no package",
    /// not "here, run the installer again".
    #[test]
    fn refuses_when_only_the_installer_is_published() {
        let assets = vec![
            asset("HoardSetup-x86_64.AppImage"),
            asset("HoardSetup-x86_64.exe"),
            asset("HoardSetup-aarch64.dmg"),
        ];
        for delivery in [Delivery::AppImage, Delivery::Nsis, Delivery::Dmg] {
            assert!(
                asset_for(delivery, &assets).is_none(),
                "{delivery:?} settled for our own installer"
            );
        }
    }
}
