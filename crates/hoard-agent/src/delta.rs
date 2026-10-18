//! zstd patches: a file encoded with an earlier version of it as the
//! reference prefix (what `zstd --patch-from` does). Fallout 4 saves 4 s to
//! 11 min apart: 21 MB of plain zstd become 0.7-1.9 MB, in 0.13-0.2 s.

use anyhow::{ensure, Result};
use sha2::{Digest, Sha256};

pub(crate) const ENCODING: &str = "zstd-patch";
pub(crate) const INFLATED: &str = hoard_deflate::ENCODING;

/// Fast, and the measured sweet spot: level 9 only shaved 10% off a patch.
const LEVEL: i32 = 3;

fn zerr(code: usize) -> anyhow::Error {
    anyhow::anyhow!("zstd: {}", zstd::zstd_safe::get_error_name(code))
}

/// Patch `new` against `base`. Both whole in memory: the reference has to be,
/// and the caller caps the sizes. One shot, so zstd doesn't buffer a second
/// copy of `new` the size of its window.
pub(crate) fn make(base: &[u8], new: &[u8]) -> Result<Vec<u8>> {
    use zstd::zstd_safe::{compress_bound, CCtx, CParameter};
    let mut cctx = CCtx::create();
    cctx.set_parameter(CParameter::CompressionLevel(LEVEL))
        .map_err(zerr)?;
    cctx.set_parameter(CParameter::WindowLog(window_log(base.len() + new.len())))
        .map_err(zerr)?;
    cctx.set_parameter(CParameter::EnableLongDistanceMatching(true))
        .map_err(zerr)?;
    cctx.ref_prefix(base).map_err(zerr)?;
    let mut out = Vec::with_capacity(compress_bound(new.len()));
    cctx.compress2(&mut out, new).map_err(zerr)?;
    Ok(out)
}

/// The `size` raw bytes `patch` rebuilds from `base`.
pub(crate) fn apply(base: &[u8], patch: &[u8], size: usize) -> Result<Vec<u8>> {
    use zstd::zstd_safe::{DCtx, DParameter};
    let mut dctx = DCtx::create();
    dctx.set_parameter(DParameter::WindowLogMax(31))
        .map_err(zerr)?;
    dctx.ref_prefix(base).map_err(zerr)?;
    let mut out = vec![0u8; size];
    let n = dctx.decompress(&mut out[..], patch).map_err(zerr)?;
    ensure!(n == size, "patch rebuilt {n} bytes, expected {size}");
    Ok(out)
}

/// A `zstd-patch-inflated` blob for `new` against `base`, checked to rebuild
/// `sha`. `None` when nothing in `new` deflates back exactly. Stellaris holds
/// ~100 MB inflated, twice over at the peak: the caller runs one at a time.
pub(crate) fn make_inflated(base: &[u8], new: &[u8], sha: &str) -> Result<Option<Vec<u8>>> {
    let base_streams = hoard_deflate::streams(base);
    if base_streams.is_empty() {
        return Ok(None);
    }
    let Some(form) = hoard_deflate::inflate(new)? else {
        return Ok(None);
    };
    let mut reference = Vec::new();
    hoard_deflate::reference(base, &base_streams, &mut reference)?;
    let frame = make(&reference, &form)?;
    let form_len = form.len();
    drop(form);
    let rebuilt = rebuild(&reference, &frame, form_len, new.len())?;
    ensure!(
        hex::encode(Sha256::digest(&rebuilt)) == sha,
        "the inflated patch doesn't rebuild the file"
    );
    Ok(Some(hoard_deflate::pack(&base_streams, &frame)))
}

/// The `size` raw bytes a `zstd-patch-inflated` blob rebuilds from `base`.
pub(crate) fn apply_inflated(base: &[u8], blob: &[u8], size: usize) -> Result<Vec<u8>> {
    let (streams, frame) = hoard_deflate::unpack(blob)?;
    let form_len = zstd::zstd_safe::get_frame_content_size(frame)
        .ok()
        .flatten()
        .filter(|n| *n <= hoard_deflate::max_form_len(size as u64))
        .ok_or_else(|| anyhow::anyhow!("an inflated patch without a usable size"))?;
    let mut reference = Vec::new();
    hoard_deflate::reference(base, &streams, &mut reference)?;
    rebuild(&reference, frame, form_len as usize, size)
}

fn rebuild(reference: &[u8], frame: &[u8], form_len: usize, size: usize) -> Result<Vec<u8>> {
    let form = apply(reference, frame, form_len)?;
    let mut out = Vec::with_capacity(size);
    hoard_deflate::deflate(&form, &mut out)?;
    ensure!(
        out.len() == size,
        "rebuilt {} bytes, expected {size}",
        out.len()
    );
    Ok(out)
}

/// What plain zstd would make of `new`, from its first 256 KiB.
pub(crate) fn plain_estimate(new: &[u8]) -> usize {
    let head = &new[..new.len().min(256 * 1024)];
    match zstd::bulk::compress(head, LEVEL) {
        Ok(z) if !head.is_empty() => {
            (new.len() as f64 * z.len() as f64 / head.len() as f64) as usize
        }
        _ => new.len(),
    }
}

/// Smallest window covering the base and the new file together.
fn window_log(n: usize) -> u32 {
    (usize::BITS - n.max(1 << 10).saturating_sub(1).leading_zeros()).clamp(10, 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hoard_deflate::testing::{deflated, zip_of};
    use hoard_deflate::Wrap;

    fn sample(len: usize, seed: u32) -> Vec<u8> {
        let mut x = seed;
        (0..len)
            .map(|i| {
                x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                if i % 7 == 0 {
                    (x >> 16) as u8
                } else {
                    (i % 251) as u8
                }
            })
            .collect()
    }

    #[test]
    fn a_patch_round_trips_and_is_small() {
        let base = sample(3 << 20, 1);
        let mut new = base.clone();
        new[1_000_000..1_000_100].fill(0xee);
        new.extend_from_slice(&sample(4096, 2));
        let patch = make(&base, &new).unwrap();
        assert!(patch.len() * 50 < new.len(), "patch {} bytes", patch.len());
        assert_eq!(apply(&base, &patch, new.len()).unwrap(), new);
    }

    #[test]
    fn the_wrong_base_or_size_fails() {
        let base = sample(1 << 20, 1);
        let new = sample(1 << 20, 3);
        let patch = make(&base, &new).unwrap();
        assert!(apply(&base, &patch, new.len() + 1).is_err());
        let other = sample(1 << 20, 4);
        assert_ne!(apply(&other, &patch, new.len()).ok(), Some(new));
    }

    /// A zip as Stellaris writes one: one deflated entry, zlib level 6.
    fn zip_save(seed: u32) -> Vec<u8> {
        let body = deflated(&sample(2 << 20, seed), 6, 8, Wrap::Raw);
        zip_of(&[("gamestate", 8, body)])
    }

    #[test]
    fn an_inflated_patch_round_trips_and_is_small() {
        let base = zip_save(1);
        let mut gamestate = sample(2 << 20, 1);
        gamestate[700_000..700_100].fill(0xee);
        let new = zip_of(&[("gamestate", 8, deflated(&gamestate, 6, 8, Wrap::Raw))]);
        let sha = hex::encode(Sha256::digest(&new));
        let blob = make_inflated(&base, &new, &sha).unwrap().unwrap();
        let plain = make(&base, &new).unwrap();
        assert!(
            blob.len() * 10 < plain.len(),
            "{} vs {}",
            blob.len(),
            plain.len()
        );
        assert_eq!(apply_inflated(&base, &blob, new.len()).unwrap(), new);
        assert_ne!(
            apply_inflated(&zip_save(2), &blob, new.len()).ok(),
            Some(new)
        );
    }

    #[test]
    fn windows_cover_both_files() {
        assert_eq!(window_log(1), 10);
        assert_eq!(window_log(1 << 20), 20);
        assert_eq!(window_log((1 << 20) + 1), 21);
        assert_eq!(window_log(usize::MAX), 31);
    }
}
