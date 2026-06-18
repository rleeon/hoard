//! Frameless web panel windows for the Pro overlay (`hoard-screen`).
//!
//! These are created here, in Rust, rather than from JS, for reasons the
//! runtime `new WebviewWindow(...)` JS API can't cover:
//!
//!   1. **Disable HTML5 fullscreen.** WebKitGTK fullscreens the *toplevel
//!      window* when a `<video>` requests it, blowing the panel up to the whole
//!      monitor and bypassing tao (so `isFullscreen()` never even sees it). We
//!      inject an initialization script that makes the fullscreen request a
//!      no-op, so the player's "maximize" stays inside the panel.
//!   2. **Track navigation.** `on_navigation` lets the overlay's address bar
//!      follow the page as the user clicks through (Google → a result → …).
//!   3. **True crop.** [`set_web_crop`] clips the window to a sub-rectangle via
//!      the X11 SHAPE extension (gdk window shape region) — the page keeps its
//!      full size (the video does NOT shrink), only the chosen rectangle is
//!      drawn over the game. Resizing the window can't do this; it reflows.
//!      Edit mode shapes the FULL window (so WebKitGTK paints every pixel);
//!      Play shapes a strict subset of it, so it never reveals an area WebKit
//!      never painted — that reveal was the torn/transparent-strip bug.
//!
//! The window itself still gets NO capabilities (it's not in any capability
//! `windows` list), so the remote page can't reach Tauri IPC. The overlay
//! window owns it and issues every geometry command against the returned label.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// Runs at document-start in the web panel (and every subframe / SPA nav). Makes
/// `requestFullscreen` a no-op so a video's "maximize" can't fullscreen the
/// toplevel window and escape the panel. Deliberately minimal — it does NOT
/// touch `document.fullscreenEnabled` or other player state, to avoid tripping
/// player initialisation.
const DISABLE_FULLSCREEN: &str = r#"
(function () {
  try {
    var noop = function () { return Promise.reject(new Error('fullscreen disabled')); };
    var silent = function () {};
    if (window.Element && Element.prototype) {
      if (Element.prototype.requestFullscreen) Element.prototype.requestFullscreen = noop;
      if (Element.prototype.webkitRequestFullscreen) Element.prototype.webkitRequestFullscreen = silent;
      if (Element.prototype.mozRequestFullScreen) Element.prototype.mozRequestFullScreen = silent;
    }
  } catch (e) {}
})();
"#;

#[derive(Clone, Serialize)]
struct NavPayload {
    label: String,
    url: String,
}

/// Create (or no-op if it already exists) a frameless, always-on-top web panel
/// at the given logical screen rect. Geometry is driven entirely by the overlay
/// afterwards via the standard window commands on this label, plus
/// [`set_web_crop`] for the visible region.
#[tauri::command]
pub async fn open_web_panel(
    app: AppHandle,
    label: String,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    if app.get_webview_window(&label).is_some() {
        return Ok(());
    }
    let parsed = tauri::Url::parse(&url).map_err(|e| format!("invalid url: {e}"))?;
    let nav_app = app.clone();
    let nav_label = label.clone();
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(parsed))
        .title("Hoard Web")
        .decorations(false)
        // Transparent so any window area the SHAPE reveals but WebKit hasn't
        // painted yet shows the game through (not opaque white). See set_web_crop.
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .resizable(true)
        .focused(false)
        .position(x, y)
        .inner_size(width, height)
        .initialization_script(DISABLE_FULLSCREEN)
        .on_navigation(move |u| {
            let _ = nav_app.emit_to(
                "overlay",
                "web://nav",
                NavPayload {
                    label: nav_label.clone(),
                    url: u.to_string(),
                },
            );
            true
        })
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Clip a web panel to a sub-rectangle of its full box (true crop) and draw the
/// edit-mode preview.
///
/// `width`/`height` are the full panel box (logical px); the window is expected
/// to already be sized to them. `top/right/bottom/left` are the per-edge crop
/// insets. In **Play** the window is shaped to the inner crop rectangle, so only
/// that shows over the game and the rest is genuinely not drawn (and clicks pass
/// through). In **Edit** the shape exposes the WHOLE box (so WebKit paints every
/// pixel and Play's smaller shape never reveals an unpainted region), and a
/// dimming "spotlight" is injected into the page to mark the region that Play
/// will show — so the user sees the full video plus the smaller rectangle,
/// exactly the mental model they described. The crop can also be displaced on
/// screen in Play (the overlay shifts the whole window by the crop offset); the
/// shape stays window-relative, so the same page region shows at a new spot.
#[tauri::command]
pub async fn set_web_crop(
    app: AppHandle,
    label: String,
    edit: bool,
    width: f64,
    height: f64,
    top: f64,
    right: f64,
    bottom: f64,
    left: f64,
) -> Result<(), String> {
    let Some(win) = app.get_webview_window(&label) else {
        return Ok(());
    };

    // --- native window clip (Linux/X11 only) ------------------------------
    // Shape the gdk window to the visible rectangle. Play → the inner crop rect;
    // Edit → the WHOLE box. Painting the full surface in Edit is what makes the
    // transition tear-free: Play only ever shapes a strict SUBSET of the box, so
    // it never reveals a region WebKitGTK never painted (that reveal left the
    // torn / transparent strips). The INPUT shape differs: in Edit it matches the
    // visible region so clicks reach the page (scroll, play — the box is moved
    // and cropped from handles drawn outside it); in Play it is empty, so the
    // panel is fully click-through and the visible crop rect is only *shown*.
    #[cfg(target_os = "linux")]
    {
        use gtk::prelude::*;
        let (vx, vy, vw, vh) = if edit {
            (0.0, 0.0, width.max(1.0), height.max(1.0))
        } else {
            (
                left,
                top,
                (width - left - right).max(1.0),
                (height - top - bottom).max(1.0),
            )
        };
        let win_for_shape = win.clone();
        win.run_on_main_thread(move || {
            if let Ok(gtk_win) = win_for_shape.gtk_window() {
                if let Some(gdk_win) = gtk_win.window() {
                    let scale = gdk_win.scale_factor() as f64;
                    let ri = |x: f64, y: f64, w: f64, h: f64| {
                        gtk::cairo::RectangleInt::new(
                            (x * scale).round() as i32,
                            (y * scale).round() as i32,
                            (w * scale).round() as i32,
                            (h * scale).round() as i32,
                        )
                    };
                    let region = gtk::cairo::Region::create_rectangle(&ri(vx, vy, vw, vh));
                    gdk_win.shape_combine_region(Some(&region), 0, 0);
                    if edit {
                        gdk_win.input_shape_combine_region(&region, 0, 0);
                    } else {
                        let empty = gtk::cairo::Region::create();
                        gdk_win.input_shape_combine_region(&empty, 0, 0);
                    }
                    // Nudge a repaint so re-revealing a region on the reverse
                    // (Play → Edit) transition redraws instead of showing stale
                    // pixels.
                    gdk_win.invalidate_rect(None, true);
                    gtk_win.queue_draw();
                }
            }
        })
        .ok();
    }

    // Edit-mode spotlight / play-mode cleanup, injected into the page.
    let js = if edit {
        format!(
            r#"(function(){{
  var id='__hoard_crop__';
  var m=document.getElementById(id);
  if(!m){{m=document.createElement('div');m.id=id;
    m.style.cssText='position:fixed;pointer-events:none;z-index:2147483647;'
      +'box-shadow:0 0 0 9999px rgba(0,0,0,0.5);'
      +'outline:2px solid #34d399;outline-offset:-2px;border-radius:4px;';
    (document.documentElement||document.body).appendChild(m);}}
  m.style.top={top}+'px';m.style.left={left}+'px';
  m.style.right={right}+'px';m.style.bottom={bottom}+'px';
}})();"#,
            top = top,
            left = left,
            right = right,
            bottom = bottom,
        )
    } else {
        "(function(){var m=document.getElementById('__hoard_crop__');if(m)m.remove();})();"
            .to_string()
    };
    let _ = win.eval(&js);
    Ok(())
}
