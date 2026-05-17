//! `hoard-server upgrade` — pulls the latest release binary from GitHub and
//! swaps it into place.
//!
//! Design notes
//! ------------
//! - This runs entirely outside the Axum/DB world. No config loading, no
//!   database connection — just an HTTP call to GitHub plus a file swap. That
//!   lets the user run `hoard-server upgrade` even when the service is broken.
//! - We do NOT restart the systemd service ourselves. Distro packaging differs
//!   (some users run it under systemd, others bare-metal); printing a one-line
//!   "now run `sudo systemctl restart hoard-server`" is portable.
//! - We do NOT touch the config file or DB. The binary swap is the entirety
//!   of the upgrade — schema migrations happen on next boot via
//!   `db::run_migrations`.

use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncReadExt;

const GH_LATEST: &str = "https://api.github.com/repos/rleeon/hoard/releases/latest";
const USER_AGENT: &str = concat!("hoard-server/", env!("CARGO_PKG_VERSION"));

#[derive(serde::Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Entry point used by `main`. Prints progress to stdout so a user running
/// this interactively can see what's happening; no tracing init is needed.
pub async fn run(target: Option<PathBuf>) -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    println!("hoard-server upgrade");
    println!("  current version: {current}");

    // 1. Ask GitHub what the latest release is.
    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()?;

    let release: Release = client
        .get(GH_LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("fetching latest release from GitHub")?
        .error_for_status()
        .context("GitHub returned a non-2xx status")?
        .json()
        .await
        .context("parsing GitHub release JSON")?;

    let latest = release.tag_name.trim_start_matches('v').to_string();
    println!("  latest version : {latest}");

    if !is_newer(&latest, current) {
        println!("\nAlready up to date.");
        return Ok(());
    }

    // 2. Find the right asset for this host. We currently ship the server
    //    only for Linux x86_64 (.tar.gz). Bail on other hosts rather than
    //    silently install the wrong artifact.
    let want = preferred_asset_name(&latest)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == want)
        .ok_or_else(|| {
            anyhow!(
                "no asset named `{want}` in release v{latest}. Available: {}",
                release
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })?;

    println!("  downloading    : {}", asset.browser_download_url);

    // 3. Download into a tempfile next to the target so the final rename is
    //    atomic and crosses no filesystem boundaries.
    let target_path = resolve_target(target)?;
    let target_dir = target_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let tmp_path = target_dir.join(format!(".hoard-server.upgrade.{}", std::process::id()));

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .context("downloading release asset")?
        .error_for_status()
        .context("download returned a non-2xx status")?
        .bytes()
        .await
        .context("reading release asset body")?;

    // 4. Extract the `hoard-server` binary out of the gzipped tarball.
    extract_binary(&bytes, &tmp_path)
        .await
        .context("extracting hoard-server from tarball")?;

    // 5. Mark executable and swap into place via rename (atomic on the same
    //    FS). Processes currently exec'ing the old binary keep their old
    //    inode on Linux; the next start picks up the new file. The chmod
    //    is a unix-only concern — on Windows the binary is .exe and the
    //    NTFS permission bits aren't expressed this way. We keep the
    //    server compiling on Windows so the CI matrix passes; the actual
    //    `upgrade` command is still gated to linux-x86_64 by the asset
    //    matcher above.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)?;
    }

    if let Err(e) = std::fs::rename(&tmp_path, &target_path) {
        // Common case: no write permission on /usr/local/bin.
        let _ = std::fs::remove_file(&tmp_path);
        bail!(
            "failed to install new binary to {}: {e}.\n\
             Try running with sudo: `sudo hoard-server upgrade`",
            target_path.display()
        );
    }

    println!(
        "\nInstalled hoard-server v{latest} to {}",
        target_path.display()
    );
    println!("\nNext step: restart the service so the new binary takes effect:");
    println!("  sudo systemctl restart hoard-server");

    Ok(())
}

/// Where to install the new binary. By default we replace the running binary,
/// resolved via `/proc/self/exe`. A `--target` flag lets users override.
fn resolve_target(override_: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = override_ {
        return Ok(p);
    }
    let exe = std::env::current_exe().context("locating current executable via /proc/self/exe")?;
    // `current_exe` can return paths like `…/hoard-server (deleted)` after a
    // swap on Linux; canonicalize to strip that suffix.
    Ok(std::fs::canonicalize(&exe).unwrap_or(exe))
}

/// Match the release-asset naming convention used by the CI release workflow.
fn preferred_asset_name(version: &str) -> Result<String> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        Ok(format!("hoard-{version}-linux-x86_64.tar.gz"))
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    {
        let _ = version;
        bail!(
            "self-upgrade is only supported on linux-x86_64 today. \
             Download the binary manually from GitHub."
        )
    }
}

/// Pull `hoard-server` out of a gzipped tarball into `dest`.
async fn extract_binary(tarball: &[u8], dest: &Path) -> Result<()> {
    use async_compression::tokio::bufread::GzipDecoder;
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;

    let decoder = GzipDecoder::new(tokio::io::BufReader::new(std::io::Cursor::new(tarball)));
    let mut archive = tokio_tar::Archive::new(decoder);
    let mut entries = archive.entries()?;

    while let Some(entry) = entries.next().await {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        // The release tarball lays things out flat, but be defensive: accept
        // any file whose final component is exactly `hoard-server`.
        if path.file_name().and_then(|s| s.to_str()) == Some("hoard-server") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).await?;
            let mut out = tokio::fs::File::create(dest)
                .await
                .with_context(|| format!("creating temp file {}", dest.display()))?;
            out.write_all(&buf).await?;
            out.flush().await?;
            return Ok(());
        }
    }
    bail!("tarball did not contain a `hoard-server` binary")
}

/// Lexicographic semver compare.
fn is_newer(latest: &str, current: &str) -> bool {
    let l = parse_version(latest);
    let c = parse_version(current);
    match (l, c) {
        (Some(l), Some(c)) => l > c,
        _ => latest != current, // any string difference counts as newer
    }
}

fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches('v');
    let core = s.split(['-', '+']).next()?;
    let mut it = core.split('.');
    Some((
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newer_basic() {
        assert!(is_newer("1.3.4", "1.3.3"));
        assert!(is_newer("1.4.0", "1.3.99"));
        assert!(!is_newer("1.3.3", "1.3.3"));
        assert!(!is_newer("1.3.2", "1.3.3"));
    }

    #[test]
    fn newer_with_v_prefix() {
        assert!(is_newer("v1.3.4", "1.3.3"));
        assert!(is_newer("v1.3.4", "v1.3.3"));
    }

    #[test]
    fn newer_double_digit() {
        assert!(is_newer("1.10.0", "1.9.0"));
        assert!(is_newer("2.0.0", "1.99.99"));
    }
}
