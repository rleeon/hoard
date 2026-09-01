//! Screen-region grab for the scope/magnifier widget: `(x, y, w, h)` in
//! virtual-desktop pixels → tightly-packed RGBA frame.
//!
//! Windows: a plain GDI `BitBlt` from the screen DC into a DIB section. The
//! scope regions are small (a few hundred px) at ~30 fps, so GDI is plenty and
//! needs none of the WGC per-window plumbing. `CAPTUREBLT` includes other
//! apps' layered windows; the overlay's OWN windows are kept out of the grab
//! by the runtime setting `WDA_EXCLUDEFROMCAPTURE` on them while a scope
//! exists, without that the lens would recursively magnify itself.
//!
//! Other platforms return `None` for now (the scope shows a dim glass
//! placeholder): X11's `GetImage` on the root would capture our own composited
//! overlay (same recursion, no exclusion mechanism), and Wayland only offers
//! the portal/PipeWire path, which is far too heavy to spin up per-lens.

#[cfg(all(target_os = "windows", feature = "runtime"))]
pub fn grab(x: i32, y: i32, w: u32, h: u32) -> Option<crate::source::Frame> {
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
        SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, HGDIOBJ,
        SRCCOPY,
    };

    if w == 0 || h == 0 {
        return None;
    }
    unsafe {
        let screen = GetDC(None);
        if screen.is_invalid() {
            return None;
        }
        let mem = CreateCompatibleDC(screen);

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w as i32,
                // Negative height = top-down rows, matching Frame's layout.
                biHeight: -(h as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let Ok(dib) = CreateDIBSection(screen, &bmi, DIB_RGB_COLORS, &mut bits, None, 0) else {
            let _ = DeleteDC(mem);
            let _ = ReleaseDC(None, screen);
            return None;
        };
        let old = SelectObject(mem, HGDIOBJ(dib.0));

        let ok = BitBlt(
            mem,
            0,
            0,
            w as i32,
            h as i32,
            screen,
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        )
        .is_ok();

        let frame = if ok && !bits.is_null() {
            let n = (w as usize) * (h as usize) * 4;
            let src = std::slice::from_raw_parts(bits as *const u8, n);
            // BGRX → RGBA (GDI leaves the X byte undefined; force opaque).
            let mut buf = vec![0u8; n];
            for (s, d) in src.chunks_exact(4).zip(buf.chunks_exact_mut(4)) {
                d[0] = s[2];
                d[1] = s[1];
                d[2] = s[0];
                d[3] = 255;
            }
            Some(crate::source::Frame::new(w, h, buf))
        } else {
            None
        };

        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ(dib.0));
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, screen);
        frame
    }
}

#[cfg(not(all(target_os = "windows", feature = "runtime")))]
pub fn grab(_x: i32, _y: i32, _w: u32, _h: u32) -> Option<crate::source::Frame> {
    None
}
