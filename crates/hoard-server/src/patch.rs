//! zstd patches: a blob encoded with an earlier blob of the same account as
//! zstd's reference prefix (`zstd-patch`), or with both inflated first
//! (`zstd-patch-inflated`, see `hoard_deflate`). Cloud stores them as they
//! come; a self-hosted server rebuilds them on the way in and stores the raw
//! file.

use std::path::Path;

/// Header naming the base of a `zstd-patch` blob on the self-hosted PUT.
pub const BASE_HEADER: &str = "x-hoard-blob-base";

pub const ENCODING: &str = "zstd-patch";
pub const INFLATED: &str = hoard_deflate::ENCODING;

pub fn is_patch(encoding: &str) -> bool {
    encoding == ENCODING || encoding == INFLATED
}

/// Rebuild `out` (`size` raw bytes) from `base` and `patch`, all on disk. The
/// base and the output are mapped, so a 90 MB save never sits in this
/// process's heap. Cloud runs it in a child process (`hoard-server
/// apply-patch`): mapped pages still count as resident, and `memwatch` watches
/// the server's.
pub fn apply_files(base: &Path, patch: &Path, out: &Path, size: u64) -> anyhow::Result<()> {
    use anyhow::Context;
    let base_file = std::fs::File::open(base).context("opening the base")?;
    // SAFETY: nothing writes these files while they're mapped (temp files, or
    // a stored blob, which never changes once it lands).
    let base_map = unsafe { memmap2::Mmap::map(&base_file) }.context("mapping the base")?;
    let patch_bytes = std::fs::read(patch).context("reading the patch")?;
    let out_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(out)
        .context("creating the output")?;
    out_file.set_len(size)?;
    if size == 0 {
        return Ok(());
    }
    // SAFETY: as above.
    let mut out_map =
        unsafe { memmap2::MmapMut::map_mut(&out_file) }.context("mapping the output")?;
    let mut dctx = zstd::zstd_safe::DCtx::create();
    let zerr = |code| anyhow::anyhow!("zstd: {}", zstd::zstd_safe::get_error_name(code));
    dctx.set_parameter(zstd::zstd_safe::DParameter::WindowLogMax(31))
        .map_err(zerr)?;
    dctx.ref_prefix(&base_map[..]).map_err(zerr)?;
    let n = dctx
        .decompress(&mut out_map[..], &patch_bytes)
        .map_err(zerr)?;
    anyhow::ensure!(n as u64 == size, "patch rebuilt {n} bytes, expected {size}");
    out_map.flush()?;
    Ok(())
}

/// Rebuild `out` from `base` and an inflated patch, at most `size_cap` bytes.
/// The base's reference and the inflated form go through mapped temp files
/// next to `out`, for the same reason as [`apply_files`]: Stellaris inflates
/// to ~100 MB.
pub fn apply_inflated_files(
    base: &Path,
    patch: &Path,
    out: &Path,
    size_cap: u64,
) -> anyhow::Result<()> {
    use anyhow::Context;
    use std::io::Write;
    let base_file = std::fs::File::open(base).context("opening the base")?;
    // SAFETY: as in `apply_files`.
    let base_map = unsafe { memmap2::Mmap::map(&base_file) }.context("mapping the base")?;
    let blob = std::fs::read(patch).context("reading the patch")?;
    let (streams, frame) = hoard_deflate::unpack(&blob)?;
    let form_len = content_size(frame)
        .filter(|n| *n <= hoard_deflate::max_form_len(size_cap))
        .context("an inflated patch without a usable size")?;

    let scratch = Scratch(vec![out.with_extension("ref"), out.with_extension("form")]);
    let (ref_path, form_path) = (&scratch.0[0], &scratch.0[1]);
    {
        let mut w = std::io::BufWriter::new(
            std::fs::File::create(ref_path).context("creating the reference")?,
        );
        hoard_deflate::reference(&base_map, &streams, &mut w)?;
        w.flush()?;
    }
    drop(base_map);
    let ref_file = std::fs::File::open(ref_path)?;
    // SAFETY: our own temp file, not written again while mapped.
    let ref_map = unsafe { memmap2::Mmap::map(&ref_file) }.context("mapping the reference")?;

    let form_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(form_path)
        .context("creating the inflated form")?;
    form_file.set_len(form_len)?;
    // SAFETY: as above.
    let mut form_map =
        unsafe { memmap2::MmapMut::map_mut(&form_file) }.context("mapping the inflated form")?;
    let mut dctx = zstd::zstd_safe::DCtx::create();
    let zerr = |code| anyhow::anyhow!("zstd: {}", zstd::zstd_safe::get_error_name(code));
    dctx.set_parameter(zstd::zstd_safe::DParameter::WindowLogMax(31))
        .map_err(zerr)?;
    dctx.ref_prefix(&ref_map[..]).map_err(zerr)?;
    let n = dctx.decompress(&mut form_map[..], frame).map_err(zerr)?;
    anyhow::ensure!(
        n as u64 == form_len,
        "patch rebuilt {n} bytes, expected {form_len}"
    );
    drop(dctx);
    drop(ref_map);

    let mut w = Capped {
        inner: std::io::BufWriter::new(std::fs::File::create(out).context("creating the output")?),
        left: size_cap,
    };
    hoard_deflate::deflate(&form_map, &mut w)?;
    w.inner.flush()?;
    Ok(())
}

/// Temp files removed however the rebuild ends.
struct Scratch(Vec<std::path::PathBuf>);

impl Drop for Scratch {
    fn drop(&mut self) {
        for p in &self.0 {
            let _ = std::fs::remove_file(p);
        }
    }
}

struct Capped<W> {
    inner: W,
    left: u64,
}

impl<W: std::io::Write> std::io::Write for Capped<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.len() as u64 > self.left {
            return Err(std::io::Error::other("rebuilt past its size"));
        }
        let n = self.inner.write(buf)?;
        self.left -= n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// The raw size a patch declares in its frame header, if it does.
pub fn content_size(patch: &[u8]) -> Option<u64> {
    zstd::zstd_safe::get_frame_content_size(patch)
        .ok()
        .flatten()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_patch_rebuilds_from_files() {
        let dir = tempfile::tempdir().unwrap();
        let base: Vec<u8> = (0..200_000u32)
            .flat_map(|i| (i % 251).to_le_bytes())
            .collect();
        let mut new = base.clone();
        new[123_456] ^= 0x5a;
        new.extend_from_slice(b"tail");
        let mut patch = Vec::new();
        {
            use std::io::Write;
            let mut enc =
                zstd::stream::write::Encoder::with_ref_prefix(&mut patch, 3, &base).unwrap();
            enc.write_all(&new).unwrap();
            enc.finish().unwrap();
        }
        let (b, p, o) = (
            dir.path().join("b"),
            dir.path().join("p"),
            dir.path().join("o"),
        );
        std::fs::write(&b, &base).unwrap();
        std::fs::write(&p, &patch).unwrap();
        super::apply_files(&b, &p, &o, new.len() as u64).unwrap();
        assert_eq!(std::fs::read(&o).unwrap(), new);
        assert!(super::apply_files(&b, &p, &o, new.len() as u64 + 1).is_err());
    }

    #[test]
    fn an_inflated_patch_rebuilds_from_files() {
        use hoard_deflate::testing::{deflated, zip_of};
        use hoard_deflate::Wrap;
        let dir = tempfile::tempdir().unwrap();
        let text: Vec<u8> = (0..400_000u32)
            .flat_map(|i| format!("{} ", i % 977).into_bytes())
            .collect();
        let base = zip_of(&[("gamestate", 8, deflated(&text, 6, 8, Wrap::Raw))]);
        let mut changed = text.clone();
        changed[50_000..50_010].fill(b'x');
        let new = zip_of(&[("gamestate", 8, deflated(&changed, 6, 8, Wrap::Raw))]);

        let streams = hoard_deflate::streams(&base);
        let mut reference = Vec::new();
        hoard_deflate::reference(&base, &streams, &mut reference).unwrap();
        let form = hoard_deflate::inflate(&new).unwrap().unwrap();
        let mut frame = Vec::new();
        {
            use std::io::Write;
            let mut enc =
                zstd::stream::write::Encoder::with_ref_prefix(&mut frame, 3, &reference).unwrap();
            enc.set_parameter(zstd::stream::raw::CParameter::WindowLog(27))
                .unwrap();
            enc.set_pledged_src_size(Some(form.len() as u64)).unwrap();
            enc.write_all(&form).unwrap();
            enc.finish().unwrap();
        }
        let blob = hoard_deflate::pack(&streams, &frame);
        assert!(blob.len() * 20 < new.len(), "{} byte patch", blob.len());

        let (b, p, o) = (
            dir.path().join("b"),
            dir.path().join("p"),
            dir.path().join("o"),
        );
        std::fs::write(&b, &base).unwrap();
        std::fs::write(&p, &blob).unwrap();
        super::apply_inflated_files(&b, &p, &o, new.len() as u64).unwrap();
        assert_eq!(std::fs::read(&o).unwrap(), new);
        assert!(!o.with_extension("ref").exists() && !o.with_extension("form").exists());
        assert!(super::apply_inflated_files(&b, &p, &o, new.len() as u64 - 1).is_err());
    }
}
