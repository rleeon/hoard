//! macOS on-screen overlay.
//!
//! A borderless, transparent `NSWindow` at a high window level with
//! `ignoresMouseEvents` (so View mode is click-through), covering the main
//! screen. Each composited frame is pushed into an `NSImageView` via an
//! `NSBitmapImageRep`. We drive the app loop manually (drain events + present +
//! IPC each tick) instead of blocking in `[NSApp run]`.
//!
//! Editor mode follows the Wayland pattern: macOS grants no app-level global
//! hotkey and we can't relocate *other* apps' windows without Accessibility, so
//! the toggle arrives over IPC (`set_editor`) and Editor simply hides the
//! overlay, the real windows are already on screen for the user to move/resize
//! natively. On leaving Editor we read each captured window's current bounds
//! from the window-server list so the panel reappears where it was left.

#![allow(non_upper_case_globals, non_snake_case, unexpected_cfgs)]

use std::ffi::c_void;
use std::io::{BufRead, Write};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use cocoa::base::{id, nil, NO, YES};
use cocoa::foundation::{NSRect, NSSize, NSString};
use objc::{class, msg_send, sel, sel_impl};

use crate::engine::Engine;
use crate::ipc::{parse_line, Message};
use crate::mode::Mode;
use crate::scene::{Panel, SourceRef};

type CFTypeRef = *const c_void;
type CFIndex = isize;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGPoint {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGSize {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

const kCGWindowListOptionOnScreenOnly: u32 = 1 << 0;
const kCGNullWindowID: u32 = 0;
const kCFNumberSInt64Type: i32 = 4;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFArrayGetCount(array: CFTypeRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, idx: CFIndex) -> CFTypeRef;
    fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFNumberGetValue(number: CFTypeRef, the_type: i32, value: *mut c_void) -> bool;
    fn CFRelease(cf: CFTypeRef);
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFTypeRef;
    fn CGRectMakeWithDictionaryRepresentation(dict: CFTypeRef, rect: *mut CGRect) -> bool;
    static kCGWindowNumber: CFTypeRef;
    static kCGWindowBounds: CFTypeRef;
}

pub fn run(mut engine: Engine) -> Result<(), String> {
    unsafe { run_inner(&mut engine) }
}

unsafe fn run_inner(engine: &mut Engine) -> Result<(), String> {
    let app: id = msg_send![class!(NSApplication), sharedApplication];
    let _: () = msg_send![app, setActivationPolicy: 1i64]; // Accessory (no dock icon)

    let screen: id = msg_send![class!(NSScreen), mainScreen];
    if screen == nil {
        return Err("no main screen".into());
    }
    let frame: NSRect = msg_send![screen, frame];
    let (sw, sh) = (frame.size.width as usize, frame.size.height as usize);

    let window: id = msg_send![class!(NSWindow), alloc];
    let window: id = msg_send![window,
        initWithContentRect: frame
        styleMask: 0u64            // NSWindowStyleMaskBorderless
        backing: 2u64              // NSBackingStoreBuffered
        defer: NO];
    let clear: id = msg_send![class!(NSColor), clearColor];
    let _: () = msg_send![window, setOpaque: NO];
    let _: () = msg_send![window, setBackgroundColor: clear];
    let _: () = msg_send![window, setLevel: 1000i64]; // ~ screen-saver level
    let _: () = msg_send![window, setIgnoresMouseEvents: YES];
    let _: () = msg_send![window, setHasShadow: NO];
    let _: () = msg_send![window, setCollectionBehavior: (1u64 << 0) | (1u64 << 4)]; // CanJoinAllSpaces | Stationary

    let view: id = msg_send![class!(NSImageView), alloc];
    let view: id = msg_send![view, initWithFrame: frame];
    let _: () = msg_send![view, setImageScaling: 0u64]; // NSImageScaleProportionallyDown unused; image is screen-sized
    let _: () = msg_send![window, setContentView: view];
    let _: () = msg_send![window, orderFrontRegardless];

    let _: () = msg_send![app, finishLaunching];

    // A reusable bitmap rep we write into each frame.
    let rep = make_rep(sw, sh);
    let bitmap_data: *mut u8 = msg_send![rep, bitmapData];

    let mut mode = Mode::View;
    let rx = spawn_stdin_reader();
    let frame_dt = Duration::from_millis(33);
    let mut buf = vec![0u8; sw * sh * 4];
    let mode_default_run_loop = NSString::alloc(nil).init_str("kCFRunLoopDefaultMode");

    loop {
        let frame_start = Instant::now();
        let pool: id = msg_send![class!(NSAutoreleasePool), new];

        // Drain pending events (non-blocking: untilDate nil).
        loop {
            let event: id = msg_send![app,
                nextEventMatchingMask: u64::MAX
                untilDate: nil
                inMode: mode_default_run_loop
                dequeue: YES];
            if event == nil {
                break;
            }
            let _: () = msg_send![app, sendEvent: event];
        }

        while let Ok(m) = rx.try_recv() {
            match m {
                Message::SetScene { scene } => engine.set_scene(scene),
                Message::GetScene => emit_scene(engine),
                Message::SetEditor { editor } => {
                    let want = if editor { Mode::Editor } else { Mode::View };
                    if want != mode {
                        set_mode(window, engine, &mut mode, want);
                        if mode == Mode::View {
                            emit_scene(engine);
                        }
                    }
                }
                Message::Quit => {
                    let _: () = msg_send![window, orderOut: nil];
                    let _: () = msg_send![pool, drain];
                    return Ok(());
                }
            }
        }

        if mode == Mode::View {
            engine.set_editing(mode == Mode::Editor);
            engine.tick();
            engine.render(&mut buf, sw as u32, sh as u32);
            present(view, rep, bitmap_data, &buf, sw, sh);
        }

        let _: () = msg_send![pool, drain];

        if let Some(rem) = frame_dt.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rem);
        }
    }
}

unsafe fn set_mode(window: id, engine: &mut Engine, mode: &mut Mode, next: Mode) {
    *mode = next;
    if next == Mode::Editor {
        // Step aside; the real windows are already on screen.
        let _: () = msg_send![window, orderOut: nil];
    } else {
        // Adopt each captured window's current on-screen bounds.
        for p in engine.scene_mut().panels.iter_mut() {
            if let Some(wid) = panel_wid(p) {
                if let Some((x, y, w, h)) = window_bounds(wid) {
                    p.rect.x = x;
                    p.rect.y = y;
                    p.rect.w = w.max(1.0);
                    p.rect.h = h.max(1.0);
                }
            }
        }
        let _: () = msg_send![window, orderFrontRegardless];
    }
}

/// Current bounds of a window by its CGWindowID, from the window-server list.
unsafe fn window_bounds(wid: u32) -> Option<(f64, f64, f64, f64)> {
    let list = CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly, kCGNullWindowID);
    if list.is_null() {
        return None;
    }
    let mut found = None;
    let count = CFArrayGetCount(list);
    for i in 0..count {
        let dict = CFArrayGetValueAtIndex(list, i);
        if dict.is_null() {
            continue;
        }
        let num = CFDictionaryGetValue(dict, kCGWindowNumber);
        if num.is_null() {
            continue;
        }
        let mut n: i64 = 0;
        if !CFNumberGetValue(num, kCFNumberSInt64Type, &mut n as *mut i64 as *mut c_void) {
            continue;
        }
        if n as u32 != wid {
            continue;
        }
        let bounds = CFDictionaryGetValue(dict, kCGWindowBounds);
        if !bounds.is_null() {
            let mut r = CGRect::default();
            if CGRectMakeWithDictionaryRepresentation(bounds, &mut r) {
                found = Some((r.origin.x, r.origin.y, r.size.width, r.size.height));
            }
        }
        break;
    }
    CFRelease(list);
    found
}

unsafe fn make_rep(w: usize, h: usize) -> id {
    let color_space = NSString::alloc(nil).init_str("NSCalibratedRGBColorSpace");
    let rep: id = msg_send![class!(NSBitmapImageRep), alloc];
    let rep: id = msg_send![rep,
        initWithBitmapDataPlanes: std::ptr::null_mut::<*mut u8>()
        pixelsWide: w as i64
        pixelsHigh: h as i64
        bitsPerSample: 8i64
        samplesPerPixel: 4i64
        hasAlpha: YES
        isPlanar: NO
        colorSpaceName: color_space
        bytesPerRow: (w * 4) as i64
        bitsPerPixel: 32i64];
    rep
}

unsafe fn present(view: id, rep: id, bitmap_data: *mut u8, rgba: &[u8], w: usize, h: usize) {
    if !bitmap_data.is_null() {
        std::ptr::copy_nonoverlapping(rgba.as_ptr(), bitmap_data, w * h * 4);
    }
    let img: id = msg_send![class!(NSImage), alloc];
    let img: id = msg_send![img, initWithSize: NSSize::new(w as f64, h as f64)];
    let _: () = msg_send![img, addRepresentation: rep];
    let _: () = msg_send![view, setImage: img];
    let _: () = msg_send![view, setNeedsDisplay: YES];
}

fn panel_wid(p: &Panel) -> Option<u32> {
    match &p.source {
        SourceRef::Window { id } => id.trim().parse().ok(),
        _ => None,
    }
}

fn emit_scene(engine: &Engine) {
    if let Ok(s) = serde_json::to_string(engine.scene()) {
        println!("{{\"type\":\"scene\",\"scene\":{s}}}");
        let _ = std::io::stdout().flush();
    }
}

fn spawn_stdin_reader() -> Receiver<Message> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            match parse_line(&line) {
                Ok(Some(msg)) => {
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!("hoard-screen: bad ipc: {e}"),
            }
        }
    });
    rx
}
