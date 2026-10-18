//! The two zlib calls this crate needs, over libz-sys.

use std::alloc::{alloc, dealloc, Layout};
use std::os::raw::{c_int, c_uint, c_void};

use anyhow::{bail, Result};
use libz_sys as z;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    /// Bare deflate, as in a zip entry.
    Raw,
    Zlib,
}

impl Wrap {
    fn window_bits(self) -> c_int {
        match self {
            Wrap::Raw => -15,
            Wrap::Zlib => 15,
        }
    }
}

const ALIGN: usize = 16;

// libz-sys types the allocator as a non-null fn, so zlib's default can't be
// asked for: each block carries its size in front for the free.
unsafe extern "C" fn zalloc(_: *mut c_void, items: c_uint, size: c_uint) -> *mut c_void {
    let Some(n) = (items as usize)
        .checked_mul(size as usize)
        .and_then(|n| n.checked_add(ALIGN))
    else {
        return std::ptr::null_mut();
    };
    let Ok(layout) = Layout::from_size_align(n, ALIGN) else {
        return std::ptr::null_mut();
    };
    let p = alloc(layout);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    p.cast::<usize>().write(n);
    p.add(ALIGN).cast()
}

unsafe extern "C" fn zfree(_: *mut c_void, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let p = ptr.cast::<u8>().sub(ALIGN);
    let n = p.cast::<usize>().read();
    dealloc(p, Layout::from_size_align_unchecked(n, ALIGN));
}

/// Boxed because zlib keeps a pointer back to it and refuses one that moved.
fn stream() -> Box<z::z_stream> {
    Box::new(z::z_stream {
        next_in: std::ptr::null_mut(),
        avail_in: 0,
        total_in: 0,
        next_out: std::ptr::null_mut(),
        avail_out: 0,
        total_out: 0,
        msg: std::ptr::null_mut(),
        state: std::ptr::null_mut(),
        zalloc,
        zfree,
        opaque: std::ptr::null_mut(),
        data_type: 0,
        adler: 0,
        reserved: 0,
    })
}

const STREAM_SIZE: c_int = std::mem::size_of::<z::z_stream>() as c_int;

struct Inflater(Box<z::z_stream>);

impl Drop for Inflater {
    fn drop(&mut self) {
        // SAFETY: initialised by `inflateInit2_` in `inflate`.
        unsafe { z::inflateEnd(&mut *self.0) };
    }
}

/// Inflate the stream `data` starts with: its bytes and how much of `data` it
/// took. `None` if it isn't one, or if it inflates past `budget`.
#[cfg(test)]
pub fn inflate(data: &[u8], wrap: Wrap, budget: usize) -> Option<(Vec<u8>, usize)> {
    let mut out = Vec::new();
    match inflate_into(data, wrap, budget, false, |p| out.extend_from_slice(p))? {
        (used, true) => Some((out, used)),
        _ => None,
    }
}

/// The first `n` inflated bytes of the stream `data` starts with, how much of
/// `data` they took, and whether the stream ended within them.
pub fn inflate_head(data: &[u8], wrap: Wrap, n: usize) -> Option<(Vec<u8>, usize, bool)> {
    let mut out = Vec::new();
    let (used, ended) = inflate_into(data, wrap, n, true, |p| out.extend_from_slice(p))?;
    Some((out, used, ended))
}

/// Inflate the stream `data` starts with into `sink`, at most `cap` bytes:
/// how much of `data` it took and whether it ended. Past `cap` it stops there
/// with `stop_at_cap`, else gives `None`, as it does for what isn't a stream.
/// What reached `sink` before a `None` stays there.
pub fn inflate_into(
    data: &[u8],
    wrap: Wrap,
    cap: usize,
    stop_at_cap: bool,
    mut sink: impl FnMut(&[u8]),
) -> Option<(usize, bool)> {
    let mut s = stream();
    // SAFETY: a fresh stream, and the version and size of the struct passed.
    let rc =
        unsafe { z::inflateInit2_(&mut *s, wrap.window_bits(), z::zlibVersion(), STREAM_SIZE) };
    if rc != z::Z_OK {
        return None;
    }
    let mut inf = Inflater(s);
    let s = &mut *inf.0;
    // zlib never writes through `next_in`.
    s.next_in = data.as_ptr().cast_mut();
    s.avail_in = c_uint::try_from(data.len()).ok()?;
    let mut produced = 0usize;
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        s.next_out = chunk.as_mut_ptr();
        s.avail_out = chunk.len() as c_uint;
        // SAFETY: both buffers outlive the call and their lengths are set above.
        let rc = unsafe { z::inflate(s, z::Z_NO_FLUSH) };
        let n = chunk.len() - s.avail_out as usize;
        if produced + n > cap {
            if !stop_at_cap {
                return None;
            }
            sink(&chunk[..cap - produced]);
            return Some((s.total_in as usize, false));
        }
        sink(&chunk[..n]);
        produced += n;
        match rc {
            z::Z_STREAM_END => return Some((s.total_in as usize, true)),
            z::Z_OK => {}
            // A truncated stream ends here as Z_BUF_ERROR.
            _ => return None,
        }
    }
}

/// Deflate `raw` at these settings, handing each piece of output to `sink`;
/// `false` from it stops early and returns `Ok(false)`. Fed a megabyte at a
/// time, the same way whether checking or deflating back: zlib's output
/// doesn't depend on it, and one path for both means it can't drift.
pub fn deflate(
    raw: &[u8],
    level: u8,
    mem: u8,
    wrap: Wrap,
    mut sink: impl FnMut(&[u8]) -> bool,
) -> Result<bool> {
    let Some(mut d) = Deflater::new(level, mem, wrap) else {
        bail!("bad deflate settings {level}/{mem}");
    };
    for piece in raw.chunks(1 << 20) {
        if !d.push(piece, false, &mut sink)? {
            return Ok(false);
        }
    }
    d.push(&[], true, &mut sink)
}

/// Whether `head`, the start of what `comp` inflates to, deflates at these
/// settings to the start of `comp`. A wrong setting shows within the first
/// block, so the whole stream never has to be inflated to rule it out.
/// `whole` when `head` is all of it.
pub fn starts_like(head: &[u8], whole: bool, comp: &[u8], level: u8, mem: u8, wrap: Wrap) -> bool {
    let Some(mut d) = Deflater::new(level, mem, wrap) else {
        return false;
    };
    let mut at = 0usize;
    let mut same = |piece: &[u8]| {
        let ok = comp.get(at..at + piece.len()) == Some(piece);
        at += piece.len();
        ok
    };
    for piece in head.chunks(1 << 16) {
        if !matches!(d.push(piece, false, &mut same), Ok(true)) {
            return false;
        }
    }
    if whole {
        return matches!(d.push(&[], true, &mut same), Ok(true)) && at == comp.len();
    }
    true
}

struct Deflater(Box<z::z_stream>);

impl Deflater {
    fn new(level: u8, mem: u8, wrap: Wrap) -> Option<Self> {
        let mut s = stream();
        // SAFETY: as in `inflate`; zlib checks the parameters itself.
        let rc = unsafe {
            z::deflateInit2_(
                &mut *s,
                c_int::from(level),
                z::Z_DEFLATED,
                wrap.window_bits(),
                c_int::from(mem),
                z::Z_DEFAULT_STRATEGY,
                z::zlibVersion(),
                STREAM_SIZE,
            )
        };
        (rc == z::Z_OK).then(|| Self(s))
    }

    /// Feed `raw`, and with `finish` end the stream. `Ok(false)` if `sink`
    /// stopped it.
    fn push(
        &mut self,
        raw: &[u8],
        finish: bool,
        sink: &mut impl FnMut(&[u8]) -> bool,
    ) -> Result<bool> {
        let s = &mut *self.0;
        s.next_in = raw.as_ptr().cast_mut();
        s.avail_in = c_uint::try_from(raw.len())?;
        let flush = if finish { z::Z_FINISH } else { z::Z_NO_FLUSH };
        let mut buf = vec![0u8; 1 << 16];
        loop {
            s.next_out = buf.as_mut_ptr();
            s.avail_out = buf.len() as c_uint;
            // SAFETY: as in `inflate`.
            let rc = unsafe { z::deflate(s, flush) };
            let n = buf.len() - s.avail_out as usize;
            if !sink(&buf[..n]) {
                return Ok(false);
            }
            match rc {
                z::Z_STREAM_END => return Ok(true),
                z::Z_OK | z::Z_BUF_ERROR => {
                    // Everything taken and nothing more to hand out.
                    if !finish && s.avail_in == 0 && s.avail_out != 0 {
                        return Ok(true);
                    }
                    if rc == z::Z_BUF_ERROR && n == 0 {
                        bail!("deflate made no progress");
                    }
                }
                rc => bail!("deflate failed ({rc})"),
            }
        }
    }
}

impl Drop for Deflater {
    fn drop(&mut self) {
        // SAFETY: initialised by `deflateInit2_` in `new`.
        unsafe { z::deflateEnd(&mut *self.0) };
    }
}

pub fn version() -> String {
    // SAFETY: zlib returns a static NUL-terminated string.
    unsafe { std::ffi::CStr::from_ptr(z::zlibVersion()) }
        .to_string_lossy()
        .into_owned()
}
