//! macOS window capture via **Core Graphics** window images
//! (`CGWindowListCopyWindowInfo` to enumerate, `CGWindowListCreateImage` to grab
//! one window's pixels). This path is synchronous, perfect for the overlay's
//! per-frame `acquire()` poll, and dramatically simpler to drive than the async,
//! delegate-based ScreenCaptureKit, which is the eventual upgrade for higher
//! frame rates. Capturing needs the Screen Recording permission (TCC); the
//! system prompts on first use. DRM windows capture black by design.
//!
//! All Core Graphics / Core Foundation entry points are declared here as raw FFI
//! against the system frameworks (linked via the `core-graphics` /
//! `core-foundation` crates) so the pixel pump doesn't hinge on any one wrapper's
//! surface.

#![allow(non_upper_case_globals, non_snake_case)]

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;

use super::{CaptureBackend, CaptureError, Result, WindowId, WindowInfo};
use crate::source::{Frame, Source};

type CFTypeRef = *const c_void;
type CFIndex = isize;
type CGWindowID = u32;

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

// CGWindowListOption / CGWindowImageOption bits we use.
const kCGWindowListOptionOnScreenOnly: u32 = 1 << 0;
const kCGWindowListExcludeDesktopElements: u32 = 1 << 4;
const kCGWindowListOptionIncludingWindow: u32 = 1 << 3;
const kCGWindowImageBoundsIgnoreFraming: u32 = 1 << 0;
const kCGNullWindowID: u32 = 0;
const kCFStringEncodingUTF8: u32 = 0x0800_0100;
const kCFNumberSInt64Type: i32 = 4;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: CFTypeRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, idx: CFIndex) -> CFTypeRef;
    fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFStringGetCStringPtr(s: CFTypeRef, encoding: u32) -> *const c_char;
    fn CFStringGetCString(s: CFTypeRef, buf: *mut c_char, size: CFIndex, encoding: u32) -> bool;
    fn CFStringGetLength(s: CFTypeRef) -> CFIndex;
    fn CFNumberGetValue(number: CFTypeRef, the_type: i32, value: *mut c_void) -> bool;
    fn CFDataGetBytePtr(data: CFTypeRef) -> *const u8;
    fn CFDataGetLength(data: CFTypeRef) -> CFIndex;
    fn CFRelease(cf: CFTypeRef);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: CGWindowID) -> CFTypeRef;
    fn CGWindowListCreateImage(
        bounds: CGRect,
        list_option: u32,
        window_id: CGWindowID,
        image_option: u32,
    ) -> CFTypeRef;
    fn CGImageGetWidth(image: CFTypeRef) -> usize;
    fn CGImageGetHeight(image: CFTypeRef) -> usize;
    fn CGImageGetBytesPerRow(image: CFTypeRef) -> usize;
    fn CGImageGetDataProvider(image: CFTypeRef) -> CFTypeRef;
    fn CGDataProviderCopyData(provider: CFTypeRef) -> CFTypeRef;

    static kCGWindowNumber: CFTypeRef;
    static kCGWindowName: CFTypeRef;
    static kCGWindowOwnerName: CFTypeRef;
    static CGRectNull: CGRect;
}

pub struct MacCapture;

impl MacCapture {
    pub fn new() -> Self {
        Self
    }
}

impl CaptureBackend for MacCapture {
    fn name(&self) -> &'static str {
        "macos/cgwindow"
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        let mut out = Vec::new();
        unsafe {
            let list = CGWindowListCopyWindowInfo(
                kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements,
                kCGNullWindowID,
            );
            if list.is_null() {
                return Ok(out);
            }
            let count = CFArrayGetCount(list);
            for i in 0..count {
                let dict = CFArrayGetValueAtIndex(list, i);
                if dict.is_null() {
                    continue;
                }
                let Some(id) = dict_number(dict, kCGWindowNumber) else {
                    continue;
                };
                let title = dict_string(dict, kCGWindowName).unwrap_or_default();
                let app = dict_string(dict, kCGWindowOwnerName).unwrap_or_default();
                // Skip the chromeless 0-size helper windows with no title and no app.
                if title.is_empty() && app.is_empty() {
                    continue;
                }
                out.push(WindowInfo {
                    id: id.to_string(),
                    title,
                    app,
                    protected: false,
                });
            }
            CFRelease(list);
        }
        Ok(out)
    }

    fn capture(&self, id: &WindowId) -> Result<Box<dyn Source>> {
        let wid: CGWindowID = id
            .trim()
            .parse()
            .map_err(|_| CaptureError::Backend(format!("bad window id: {id}")))?;
        Ok(Box::new(CgWindowSource {
            id: id.clone(),
            wid,
            last: None,
        }))
    }
}

/// Read a `CFString` dictionary value as a Rust `String`.
unsafe fn dict_string(dict: CFTypeRef, key: CFTypeRef) -> Option<String> {
    let val = CFDictionaryGetValue(dict, key);
    if val.is_null() {
        return None;
    }
    let ptr = CFStringGetCStringPtr(val, kCFStringEncodingUTF8);
    if !ptr.is_null() {
        return Some(CStr::from_ptr(ptr).to_string_lossy().into_owned());
    }
    let len = CFStringGetLength(val);
    let cap = (len * 4 + 1).max(1) as usize;
    let mut buf = vec![0i8; cap];
    if CFStringGetCString(val, buf.as_mut_ptr(), cap as CFIndex, kCFStringEncodingUTF8) {
        let cstr = CStr::from_ptr(buf.as_ptr());
        Some(cstr.to_string_lossy().into_owned())
    } else {
        None
    }
}

/// Read a `CFNumber` dictionary value as an `i64`.
unsafe fn dict_number(dict: CFTypeRef, key: CFTypeRef) -> Option<i64> {
    let val = CFDictionaryGetValue(dict, key);
    if val.is_null() {
        return None;
    }
    let mut n: i64 = 0;
    if CFNumberGetValue(val, kCFNumberSInt64Type, &mut n as *mut i64 as *mut c_void) {
        Some(n)
    } else {
        None
    }
}

/// A live source backed by repeated `CGWindowListCreateImage` grabs.
struct CgWindowSource {
    id: String,
    wid: CGWindowID,
    last: Option<Frame>,
}

impl Source for CgWindowSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn acquire(&mut self) -> Option<Frame> {
        unsafe {
            let image = CGWindowListCreateImage(
                CGRectNull,
                kCGWindowListOptionIncludingWindow,
                self.wid,
                kCGWindowImageBoundsIgnoreFraming,
            );
            if image.is_null() {
                return self.last.clone();
            }
            let w = CGImageGetWidth(image);
            let h = CGImageGetHeight(image);
            let stride = CGImageGetBytesPerRow(image);
            if w == 0 || h == 0 {
                CFRelease(image);
                return self.last.clone();
            }
            let provider = CGImageGetDataProvider(image);
            if provider.is_null() {
                CFRelease(image);
                return self.last.clone();
            }
            let data = CGDataProviderCopyData(provider);
            if data.is_null() {
                CFRelease(image);
                return self.last.clone();
            }
            let bytes = CFDataGetBytePtr(data);
            let dlen = CFDataGetLength(data) as usize;

            let mut rgba = vec![0u8; w * h * 4];
            // CGWindowListCreateImage gives 32-bit BGRA (little-endian ARGB),
            // premultiplied; rows padded to `stride`.
            for y in 0..h {
                let row = y * stride;
                if row + w * 4 > dlen {
                    break;
                }
                for x in 0..w {
                    let s = row + x * 4;
                    let b = *bytes.add(s);
                    let g = *bytes.add(s + 1);
                    let r = *bytes.add(s + 2);
                    let i = (y * w + x) * 4;
                    rgba[i] = r;
                    rgba[i + 1] = g;
                    rgba[i + 2] = b;
                    rgba[i + 3] = 0xff;
                }
            }
            CFRelease(data);
            CFRelease(image);

            let frame = Frame::new(w as u32, h as u32, rgba);
            self.last = Some(frame.clone());
            Some(frame)
        }
    }
}
