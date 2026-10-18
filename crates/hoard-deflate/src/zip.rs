//! Where the entries of a zip keep their data, from its central directory.

fn u16_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u16::from_le_bytes(d.get(at..at + 2)?.try_into().ok()?).into())
}

fn u32_at(d: &[u8], at: usize) -> Option<u64> {
    Some(u32::from_le_bytes(d.get(at..at + 4)?.try_into().ok()?).into())
}

/// (offset, length, method) of each entry's data. Empty for anything that
/// isn't a plain zip, zip64 included: no save needs it.
pub fn entries(d: &[u8]) -> Vec<(u64, u64, u64)> {
    try_entries(d).unwrap_or_default()
}

fn try_entries(d: &[u8]) -> Option<Vec<(u64, u64, u64)>> {
    if d.len() < 22 || !d.starts_with(b"PK\x03\x04") {
        return None;
    }
    let from = d.len().saturating_sub(22 + 0xFFFF);
    let eocd = (from..=d.len() - 22)
        .rev()
        .find(|&i| d[i..].starts_with(b"PK\x05\x06"))?;
    let count = u16_at(d, eocd + 10)?;
    let mut at = usize::try_from(u32_at(d, eocd + 16)?).ok()?;
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        if !d.get(at..)?.starts_with(b"PK\x01\x02") {
            return None;
        }
        let method = u16_at(d, at + 10)?;
        let len = u32_at(d, at + 20)?;
        let local = u32_at(d, at + 42)?;
        if len == 0xFFFF_FFFF || local == 0xFFFF_FFFF {
            return None;
        }
        let l = usize::try_from(local).ok()?;
        if !d.get(l..)?.starts_with(b"PK\x03\x04") {
            return None;
        }
        // The local header's own name and extra lengths, which may differ
        // from the directory's.
        let start = local + 30 + u16_at(d, l + 26)? + u16_at(d, l + 28)?;
        if start + len > d.len() as u64 {
            return None;
        }
        out.push((start, len, method));
        at += 46 + (u16_at(d, at + 28)? + u16_at(d, at + 30)? + u16_at(d, at + 32)?) as usize;
    }
    Some(out)
}
