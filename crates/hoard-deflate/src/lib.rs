//! A zip save changes everywhere after its first changed byte once deflated,
//! so a patch against the previous one saves nothing: Stellaris, 8.5 MB a
//! version, patches to 8.3 MB. Inflated, the two versions share most of their
//! content and the patch drops to 0.7-1.0 MB (Factorio: 8.8 MB to 0.6-0.8).
//!
//! The patch (`zstd-patch-inflated`) is made from the base's [`reference`] to
//! the new file's [`inflate`]d form, and is only worth anything if
//! [`deflate`] then gives back the exact original bytes. It does when the
//! game used zlib and some level and memory setting reproduce each stream,
//! which this checks per stream before inflating it. A stream that doesn't
//! reproduce stays deflated.

use std::io::Write;

use anyhow::{bail, ensure, Context, Result};

mod zip;
mod zlib;

pub use zlib::Wrap;

pub const ENCODING: &str = "zstd-patch-inflated";

/// Inflated bytes allowed per file: a zip bomb stops here.
pub const MAX_INFLATED: usize = 256 << 20;

/// A deflate stream at `offset`, `len` bytes long.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stream {
    pub offset: u64,
    pub len: u64,
    pub wrap: Wrap,
}

/// The deflate streams a zip holds: its deflated entries, and zlib streams
/// stored as entries (Factorio keeps its level that way).
pub fn streams(d: &[u8]) -> Vec<Stream> {
    let mut out: Vec<Stream> = zip::entries(d)
        .into_iter()
        .filter_map(|(offset, len, method)| {
            let wrap = match method {
                8 => Wrap::Raw,
                0 if zlib_header(&d[offset as usize..(offset + len) as usize]) => Wrap::Zlib,
                _ => return None,
            };
            Some(Stream { offset, len, wrap })
        })
        .collect();
    out.sort_by_key(|s| s.offset);
    // Overlapping entries are a crafted zip, not a save.
    let mut end = 0;
    out.retain(|s| {
        let ok = s.offset >= end;
        if ok {
            end = s.offset + s.len;
        }
        ok
    });
    out
}

fn zlib_header(d: &[u8]) -> bool {
    d.len() >= 2
        && d[0] & 0x0F == 8
        && d[0] >> 4 <= 7
        && d[1] & 0x20 == 0
        && (u16::from(d[0]) << 8 | u16::from(d[1])) % 31 == 0
}

/// `d` with `streams` inflated in place: what a patch is made against. Only
/// ever inflates, so any zlib gives the same bytes. `streams` may come from a
/// patch blob, so it's checked against `d` rather than trusted.
pub fn reference(d: &[u8], streams: &[Stream], out: &mut impl Write) -> Result<()> {
    let mut at = 0usize;
    let mut budget = MAX_INFLATED;
    for s in streams {
        let start = usize::try_from(s.offset).ok();
        let end = s
            .offset
            .checked_add(s.len)
            .and_then(|e| usize::try_from(e).ok());
        let (Some(start), Some(end)) = (start, end) else {
            bail!("stream outside the file");
        };
        ensure!(start >= at && end <= d.len(), "stream outside the file");
        out.write_all(&d[at..start])?;
        let mut produced = 0usize;
        let mut failed = None;
        let got = zlib::inflate_into(&d[start..end], s.wrap, budget, false, |piece| {
            produced += piece.len();
            if failed.is_none() {
                failed = out.write_all(piece).err();
            }
        });
        if let Some(e) = failed {
            return Err(e.into());
        }
        budget = budget.saturating_sub(produced);
        // Not a stream after all, or past the budget: what came out stays,
        // followed by the bytes as they are. Both sides do the same.
        if got.is_none() {
            out.write_all(&d[start..end])?;
        }
        at = end;
    }
    out.write_all(&d[at..])?;
    Ok(())
}

// ---- the inflated form
//
// "HDF1", then pieces: 0, u64 length, bytes kept as they are; or 1, level,
// memory level, wrap (0 bare, 1 zlib), u64 deflated length, u64 inflated
// length, the inflated bytes. 0xFF ends it. Frozen: a patch made with it has
// to rebuild for as long as the version is kept.

const FORM: &[u8; 4] = b"HDF1";
const KEPT: u8 = 0;
const INFLATED: u8 = 1;
const END: u8 = 0xFF;

/// The order saves use them in. Level 0 is stored blocks, kept as they are.
const LEVELS: [u8; 9] = [6, 9, 1, 5, 4, 3, 2, 7, 8];

/// Inflated bytes each setting is first tried on. Victoria 3 and EU5 streams
/// (133 and 213 MB inflated) match none and are ruled out within this.
const PROBE: usize = 4 << 20;

/// `d` with every stream zlib re-deflates bit for bit inflated, plus what it
/// takes to deflate it back. `None` when no stream qualifies.
pub fn inflate(d: &[u8]) -> Result<Option<Vec<u8>>> {
    let found = streams(d);
    if found.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(d.len().saturating_mul(4).min(MAX_INFLATED));
    out.extend_from_slice(FORM);
    let mut at = 0usize;
    let mut budget = MAX_INFLATED;
    let mut hits = 0usize;
    // Entries of one file share their settings: try the last ones first.
    let mut last = (6u8, 8u8);
    for s in &found {
        let start = s.offset as usize;
        let comp = &d[start..(s.offset + s.len) as usize];
        let Some((head, used, whole)) = zlib::inflate_head(comp, s.wrap, PROBE.min(budget)) else {
            continue;
        };
        let probed = if whole { &comp[..used] } else { comp };
        let tries = std::iter::once(last).chain(
            LEVELS
                .iter()
                .flat_map(|&l| [(l, 8), (l, 9)])
                .filter(|&t| t != last),
        );
        // The piece goes in with a blank header, filled in once a setting
        // checks out, and comes out again if none does.
        let mark = out.len();
        kept(&mut out, &d[at..start]);
        let header = out.len();
        out.extend_from_slice(&[INFLATED, 0, 0, (s.wrap == Wrap::Zlib) as u8]);
        out.extend_from_slice(&[0; 16]);
        let body = out.len();
        // A stream that fit in the probe is checked whole by it. A bigger one
        // is inflated once, on the first setting that gets past the probe, and
        // checked whole against each that does.
        let mut inflated = whole.then_some(used);
        if whole {
            out.extend_from_slice(&head);
        }
        let mut hit = None;
        for (l, m) in tries {
            if !zlib::starts_like(&head, whole, probed, l, m, s.wrap) {
                continue;
            }
            if whole {
                hit = Some((l, m));
                break;
            }
            if inflated.is_none() {
                match zlib::inflate_into(comp, s.wrap, budget, false, |p| out.extend_from_slice(p))
                {
                    Some((used, true)) => inflated = Some(used),
                    _ => break,
                }
            }
            let used = inflated.unwrap_or_default();
            if deflates_to(&out[body..], &comp[..used], l, m, s.wrap) {
                hit = Some((l, m));
                break;
            }
        }
        drop(head);
        let (Some((level, mem)), Some(used)) = (hit, inflated) else {
            out.truncate(mark);
            continue;
        };
        let raw_len = out.len() - body;
        out[header + 1] = level;
        out[header + 2] = mem;
        out[header + 4..header + 12].copy_from_slice(&(used as u64).to_le_bytes());
        out[header + 12..header + 20].copy_from_slice(&(raw_len as u64).to_le_bytes());
        last = (level, mem);
        budget -= raw_len;
        hits += 1;
        at = start + used;
    }
    if hits == 0 {
        return Ok(None);
    }
    kept(&mut out, &d[at..]);
    out.push(END);
    Ok(Some(out))
}

fn kept(out: &mut Vec<u8>, bytes: &[u8]) {
    if !bytes.is_empty() {
        out.push(KEPT);
        out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(bytes);
    }
}

/// Whether zlib at `level`/`mem` turns `raw` back into exactly `comp`.
fn deflates_to(raw: &[u8], comp: &[u8], level: u8, mem: u8, wrap: Wrap) -> bool {
    let mut at = 0usize;
    let done = zlib::deflate(raw, level, mem, wrap, |piece| {
        let same = comp.get(at..at + piece.len()) == Some(piece);
        at += piece.len();
        same
    });
    matches!(done, Ok(true)) && at == comp.len()
}

/// Back from what [`inflate`] made to the original bytes.
pub fn deflate(mut form: &[u8], out: &mut impl Write) -> Result<()> {
    ensure!(take(&mut form, 4)? == FORM, "not an inflated form");
    loop {
        match take(&mut form, 1)?[0] {
            KEPT => {
                let n = take_len(&mut form)?;
                out.write_all(take(&mut form, n)?)?;
            }
            INFLATED => {
                let p = take(&mut form, 3)?;
                let wrap = match p[2] {
                    0 => Wrap::Raw,
                    1 => Wrap::Zlib,
                    w => bail!("unknown wrap {w}"),
                };
                let comp_len = take_len(&mut form)?;
                let n = take_len(&mut form)?;
                let raw = take(&mut form, n)?;
                let mut wrote = 0usize;
                let mut failed = None;
                zlib::deflate(raw, p[0], p[1], wrap, |piece| {
                    wrote += piece.len();
                    if wrote > comp_len {
                        return false;
                    }
                    match out.write_all(piece) {
                        Ok(()) => true,
                        Err(e) => {
                            failed = Some(e);
                            false
                        }
                    }
                })?;
                if let Some(e) = failed {
                    return Err(e.into());
                }
                ensure!(
                    wrote == comp_len,
                    "deflated back to {wrote} bytes, not {comp_len}"
                );
            }
            END => return Ok(()),
            t => bail!("unknown piece {t}"),
        }
    }
}

fn take<'a>(d: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    ensure!(d.len() >= n, "cut short");
    let (head, rest) = d.split_at(n);
    *d = rest;
    Ok(head)
}

fn take_len(d: &mut &[u8]) -> Result<usize> {
    let b = take(d, 8)?;
    usize::try_from(u64::from_le_bytes(b.try_into()?)).context("length")
}

/// The most an inflated form of a `file_len`-byte file can take: the file,
/// what its streams inflate to, and a piece header for each of them.
pub fn max_form_len(file_len: u64) -> u64 {
    file_len
        .saturating_mul(2)
        .saturating_add(MAX_INFLATED as u64 + 64)
}

// ---- the patch blob
//
// "HDP1", u32 stream count, then per stream of the base: u64 offset, u64
// length, wrap; then the zstd frame. The restoring side rebuilds the base's
// reference from these without searching for anything.

const BLOB: &[u8; 4] = b"HDP1";

pub fn pack(base_streams: &[Stream], frame: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + base_streams.len() * 17 + frame.len());
    out.extend_from_slice(BLOB);
    out.extend_from_slice(&(base_streams.len() as u32).to_le_bytes());
    for s in base_streams {
        out.extend_from_slice(&s.offset.to_le_bytes());
        out.extend_from_slice(&s.len.to_le_bytes());
        out.push((s.wrap == Wrap::Zlib) as u8);
    }
    out.extend_from_slice(frame);
    out
}

/// The base's streams and the zstd frame of a patch blob.
pub fn unpack(mut blob: &[u8]) -> Result<(Vec<Stream>, &[u8])> {
    ensure!(take(&mut blob, 4)? == BLOB, "not an inflated patch");
    let n = u32::from_le_bytes(take(&mut blob, 4)?.try_into()?) as usize;
    ensure!(n <= blob.len() / 17, "more streams than the blob holds");
    let mut streams = Vec::with_capacity(n);
    for _ in 0..n {
        let offset = u64::from_le_bytes(take(&mut blob, 8)?.try_into()?);
        let len = u64::from_le_bytes(take(&mut blob, 8)?.try_into()?);
        let wrap = match take(&mut blob, 1)?[0] {
            0 => Wrap::Raw,
            1 => Wrap::Zlib,
            w => bail!("unknown wrap {w}"),
        };
        streams.push(Stream { offset, len, wrap });
    }
    Ok((streams, blob))
}

pub fn zlib_version() -> String {
    zlib::version()
}

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::*;

    pub fn deflated(raw: &[u8], level: u8, mem: u8, wrap: Wrap) -> Vec<u8> {
        let mut out = Vec::new();
        zlib::deflate(raw, level, mem, wrap, |p| {
            out.extend_from_slice(p);
            true
        })
        .unwrap();
        out
    }

    /// A zip by hand: (name, method, data as stored).
    pub fn zip_of(entries: &[(&str, u16, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut dir = Vec::new();
        for (name, method, data) in entries {
            let offset = out.len() as u32;
            out.extend_from_slice(b"PK\x03\x04");
            out.extend_from_slice(&[20, 0, 0, 0]);
            out.extend_from_slice(&method.to_le_bytes());
            out.extend_from_slice(&[0; 8]);
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&[0; 4]);
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0; 2]);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
            dir.extend_from_slice(b"PK\x01\x02");
            dir.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
            dir.extend_from_slice(&method.to_le_bytes());
            dir.extend_from_slice(&[0; 8]);
            dir.extend_from_slice(&(data.len() as u32).to_le_bytes());
            dir.extend_from_slice(&[0; 4]);
            dir.extend_from_slice(&(name.len() as u16).to_le_bytes());
            dir.extend_from_slice(&[0; 12]);
            dir.extend_from_slice(&offset.to_le_bytes());
            dir.extend_from_slice(name.as_bytes());
        }
        let cd = out.len() as u32;
        out.extend_from_slice(&dir);
        out.extend_from_slice(b"PK\x05\x06");
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd.to_le_bytes());
        out.extend_from_slice(&[0; 2]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{deflated, zip_of};
    use super::*;

    fn text(len: usize, seed: u32) -> Vec<u8> {
        let mut x = seed;
        let words = [
            &b"fleet "[..],
            b"planet ",
            b"empire ",
            b"= { ",
            b"} ",
            b"\n\t",
        ];
        let mut out = Vec::with_capacity(len);
        while out.len() < len {
            x = x.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            out.extend_from_slice(words[(x >> 16) as usize % words.len()]);
            out.extend_from_slice((x >> 20).to_string().as_bytes());
        }
        out.truncate(len);
        out
    }

    /// One stored deflate block: valid, and no level from 1 to 9 makes it.
    fn stored_block(raw: &[u8]) -> Vec<u8> {
        let mut out = vec![1];
        out.extend_from_slice(&(raw.len() as u16).to_le_bytes());
        out.extend_from_slice(&(!(raw.len() as u16)).to_le_bytes());
        out.extend_from_slice(raw);
        out
    }

    fn save(seed: u32) -> Vec<u8> {
        zip_of(&[
            ("meta", 8, deflated(&text(4_000, seed), 6, 8, Wrap::Raw)),
            (
                "gamestate",
                8,
                deflated(&text(300_000, seed + 1), 9, 9, Wrap::Raw),
            ),
            (
                "level.dat0",
                0,
                deflated(&text(200_000, seed + 2), 1, 8, Wrap::Zlib),
            ),
            ("odd", 8, stored_block(&text(3_000, seed + 3))),
            ("readme", 0, b"plain".to_vec()),
        ])
    }

    #[test]
    fn a_zip_deflates_back_bit_for_bit() {
        let zip = save(1);
        let s = streams(&zip);
        assert_eq!(
            s.len(),
            4,
            "three deflated entries and one zlib inside a stored one"
        );
        let form = inflate(&zip).unwrap().unwrap();
        assert!(form.len() > 400_000, "inflated: {} bytes", form.len());
        let mut back = Vec::new();
        deflate(&form, &mut back).unwrap();
        assert_eq!(back, zip);
    }

    #[test]
    fn a_stream_zlib_cant_redo_stays_deflated() {
        let raw = text(3_000, 9);
        let zip = zip_of(&[("odd", 8, stored_block(&raw))]);
        assert_eq!(streams(&zip).len(), 1);
        assert_eq!(inflate(&zip).unwrap(), None);
        // The reference still inflates it: nothing there has to deflate back.
        let mut r = Vec::new();
        reference(&zip, &streams(&zip), &mut r).unwrap();
        assert!(r.windows(raw.len()).any(|w| w == raw));
    }

    #[test]
    fn what_isnt_a_zip_has_no_streams() {
        assert!(streams(b"just a save").is_empty());
        assert_eq!(inflate(&text(10_000, 3)).unwrap(), None);
        let mut cut = save(1);
        cut.truncate(cut.len() - 30);
        assert!(streams(&cut).is_empty(), "no central directory, no streams");
    }

    #[test]
    fn a_bomb_stops_at_the_budget() {
        let bomb = deflated(&vec![0u8; 8 << 20], 9, 9, Wrap::Raw);
        assert!(zlib::inflate(&bomb, Wrap::Raw, 1 << 20).is_none());
        assert_eq!(
            zlib::inflate(&bomb, Wrap::Raw, 8 << 20).unwrap().0.len(),
            8 << 20
        );
    }

    #[test]
    fn a_damaged_form_fails_instead_of_writing_junk() {
        let form = inflate(&save(2)).unwrap().unwrap();
        assert!(deflate(&form[..form.len() - 1], &mut Vec::new()).is_err());
        let mut bent = form.clone();
        let at = bent.len() / 2;
        bent[at] ^= 0xFF;
        let mut back = Vec::new();
        if deflate(&bent, &mut back).is_ok() {
            assert_ne!(back, save(2));
        }
    }

    #[test]
    fn a_stream_table_from_outside_is_checked() {
        let zip = save(1);
        let bad = |offset: u64, len: u64| {
            reference(
                &zip,
                &[Stream {
                    offset,
                    len,
                    wrap: Wrap::Raw,
                }],
                &mut Vec::new(),
            )
        };
        assert!(bad(u64::MAX - 5, 10).is_err());
        assert!(bad(10, zip.len() as u64).is_err());
        assert!(bad(zip.len() as u64 + 1, 0).is_err());
    }

    #[test]
    fn a_blob_carries_the_base_streams() {
        let s = streams(&save(1));
        let blob = pack(&s, b"frame");
        let (got, frame) = unpack(&blob).unwrap();
        assert_eq!((got, frame), (s, &b"frame"[..]));
        assert!(unpack(&blob[..10]).is_err());
        assert!(unpack(b"zstd frame").is_err());
    }

    /// zlib's output for fixed settings, from Python's zlib 1.3.1. If a libz-sys
    /// bump changes it, every inflated patch already stored stops restoring.
    #[test]
    fn zlib_still_deflates_the_same() {
        use std::fmt::Write as _;
        // Three times the piece `zlib::deflate` feeds, so the pieces are tested too.
        let raw = text(3 << 20, 42);
        let digest = |b: &[u8]| {
            // FNV-1a: enough to tell outputs apart, and no hash crate for it.
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for &x in b {
                h = (h ^ u64::from(x)).wrapping_mul(0x100_0000_01b3);
            }
            let mut s = String::new();
            let _ = write!(s, "{h:016x}");
            s
        };
        let got: Vec<String> = [(6, 8, Wrap::Raw), (9, 9, Wrap::Raw), (1, 8, Wrap::Zlib)]
            .iter()
            .map(|&(l, m, w)| digest(&deflated(&raw, l, m, w)))
            .collect();
        assert_eq!(got, GOLDEN, "zlib {}", zlib_version());
    }

    const GOLDEN: [&str; 3] = ["b60bd44ad84c56e9", "5bc6177131aeee60", "c2bdbda8e52a17c8"];
}
