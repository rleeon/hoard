use anyhow::{anyhow, bail, Context, Result};
use async_compression::tokio::bufread::ZstdDecoder;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use tokio::io::{AsyncReadExt, BufReader};
use tokio_util::io::StreamReader;

use crate::api::{ApiClient, SnapshotFile};
use crate::config::CliConfig;
use crate::state::CliState;

pub async fn apply(
    save_id: String,
    version: Option<i64>,
    to: Option<PathBuf>,
    no_verify: bool,
    force: bool,
) -> Result<()> {
    let (cfg, _) = CliConfig::load_default()?;
    let token = cfg.require_token()?;
    let client = ApiClient::new(cfg.server.url.clone(), token)?;
    // Resolve version
    let version = match version {
        Some(v) => v,
        None => {
            let save = client.get_save(&save_id).await?;
            save.latest_version_num
                .ok_or_else(|| anyhow!("save has no snapshots yet"))?
        }
    };

    // Resolve destination
    let dest = match to {
        Some(p) => p,
        None => {
            let (state, _) = CliState::load_default()?;
            state
                .saves
                .get(&save_id)
                .map(|s| s.local_path.clone())
                .ok_or_else(|| {
                    anyhow!("no remembered local path for save {save_id}; pass --to <PATH>")
                })?
        }
    };

    // Get manifest for SHA verification
    let detail = client.snapshot_detail(&save_id, version).await?;
    let expected: HashMap<String, &SnapshotFile> = detail
        .files
        .iter()
        .map(|f| (f.relative_path.clone(), f))
        .collect();

    // Prepare destination
    if dest.exists() {
        let empty = std::fs::read_dir(&dest)?.next().is_none();
        if !empty && !force {
            bail!(
                "destination is not empty: {} (pass --force to extract anyway)",
                dest.display()
            );
        }
    } else {
        std::fs::create_dir_all(&dest).with_context(|| format!("creating {}", dest.display()))?;
    }

    println!(
        "restoring v{} of {} to {} ({} files, {})",
        version,
        save_id,
        dest.display(),
        detail.snapshot.file_count,
        fmt_bytes(detail.snapshot.total_size_bytes as u64),
    );

    // Stream download → decode zstd → untar
    let resp = client.snapshot_download(&save_id, version).await?;
    let total = resp.content_length();
    let pb = match total {
        Some(n) => ProgressBar::new(n),
        None => ProgressBar::new_spinner(),
    };
    pb.set_style(
        ProgressStyle::with_template("{spinner} {wide_bar} {bytes} ({bytes_per_sec})")
            .unwrap()
            .progress_chars("=> "),
    );
    let pb_for_stream = pb.clone();

    let byte_stream = resp.bytes_stream().map(move |chunk| {
        let chunk = chunk.map_err(std::io::Error::other)?;
        pb_for_stream.inc(chunk.len() as u64);
        Ok::<_, std::io::Error>(chunk)
    });

    let reader = StreamReader::new(byte_stream);
    let buf = BufReader::new(reader);
    let zstd = ZstdDecoder::new(buf);
    let zstd = BufReader::new(zstd);
    let mut archive = tokio_tar::Archive::new(zstd);

    let mut entries = archive.entries().context("opening tar archive")?;
    let mut extracted = 0usize;

    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("reading tar entry")?;
        let path_in_tar = entry.path()?.into_owned();

        // Sanitize: reject anything that escapes the destination. For
        // directory entries an empty result (e.g. "./") just means the
        // archive root — nothing to do.
        let safe_rel = match sanitize(&path_in_tar) {
            Some(p) => p,
            None if entry.header().entry_type().is_dir() => continue,
            None => bail!("unsafe path in archive: {}", path_in_tar.display()),
        };
        let dest_path = dest.join(&safe_rel);

        if entry.header().entry_type().is_dir() {
            tokio::fs::create_dir_all(&dest_path).await.ok();
            continue;
        }
        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("creating parent {}", parent.display()))?;
        }

        let key = safe_rel.to_string_lossy().replace('\\', "/");
        let expected_file = expected.get(&key);

        // Read into memory while computing SHA so we can verify before writing.
        // For very large files we could pipe through a hasher writer; this is
        // simpler and matches the upload-side full-read approach.
        let mut bytes = Vec::with_capacity(entry.header().size().unwrap_or(0) as usize);
        entry
            .read_to_end(&mut bytes)
            .await
            .with_context(|| format!("reading entry {key}"))?;

        if !no_verify {
            if let Some(meta) = expected_file {
                let mut hasher = Sha256::new();
                hasher.update(&bytes);
                let got = hex::encode(hasher.finalize());
                if got != meta.sha256 {
                    bail!(
                        "sha256 mismatch for {key}: expected {}, got {}",
                        meta.sha256,
                        got
                    );
                }
                if (bytes.len() as i64) != meta.size_bytes {
                    bail!(
                        "size mismatch for {key}: expected {}, got {}",
                        meta.size_bytes,
                        bytes.len()
                    );
                }
            }
            // If the file isn't in the manifest, we still extract it but
            // can't verify (shouldn't happen for snapshots from this server).
        }

        tokio::fs::write(&dest_path, &bytes)
            .await
            .with_context(|| format!("writing {}", dest_path.display()))?;
        extracted += 1;
    }

    pb.finish_with_message("done");
    println!("restored {} files to {}", extracted, dest.display());
    Ok(())
}

/// Reject absolute paths, `..`, drive prefixes. Returns a relative PathBuf
/// composed of only Normal components.
fn sanitize(p: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::Normal(s) => out.push(s),
            // Skip leading `./`
            Component::CurDir => {}
            _ => return None,
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn fmt_bytes(b: u64) -> String {
    let b = b as f64;
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if b >= GB {
        format!("{:.2} GiB", b / GB)
    } else if b >= MB {
        format!("{:.2} MiB", b / MB)
    } else if b >= KB {
        format!("{:.2} KiB", b / KB)
    } else {
        format!("{} B", b as u64)
    }
}
