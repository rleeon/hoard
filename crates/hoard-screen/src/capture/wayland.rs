//! Real Wayland window capture (`feature = "wayland"`).
//!
//! The only sanctioned path on Wayland: `xdg-desktop-portal`'s **ScreenCast**
//! interface negotiates a capture (the *compositor* shows the window picker and
//! the permission prompt, we never reach into another client, which Wayland
//! forbids), then hands back a **PipeWire** node id plus an fd to the PipeWire
//! daemon. We connect PipeWire to that fd, attach an input stream to the node,
//! and copy each frame into RGBA.
//!
//! This covers compositors with no `wlr-layer-shell` (GNOME) for the *capture*
//! half; the *placement* half (an actual always-on-top surface over the game)
//! is the window layer's job, gamescope for the universal/GNOME case.
//!
//! Because the portal pops the compositor's own dialog, `capture` ignores the
//! requested id and streams whatever the user picks.

use std::cell::RefCell;
use std::os::fd::OwnedFd;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use super::{CaptureBackend, CaptureError, Result, WindowId, WindowInfo};
use crate::source::{Frame, Source};

fn be<E: std::fmt::Display>(e: E) -> CaptureError {
    CaptureError::Backend(e.to_string())
}

pub struct WaylandPortalCapture;

impl CaptureBackend for WaylandPortalCapture {
    fn name(&self) -> &'static str {
        "linux/wayland-portal-pipewire"
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        // The compositor's portal dialog does the picking; surface one entry so
        // the editor can offer a "capture a window…" action.
        Ok(vec![WindowInfo {
            id: "portal".into(),
            title: "Pick a window (system dialog)".into(),
            app: "xdg-desktop-portal".into(),
            protected: false,
        }])
    }

    fn capture(&self, _id: &WindowId) -> Result<Box<dyn Source>> {
        Ok(Box::new(WaylandSource::start()?))
    }
}

struct Shared {
    frame: Mutex<Option<Frame>>,
}

/// A capture stream fed by the PipeWire loop running on its own thread.
pub struct WaylandSource {
    id: String,
    shared: Arc<Shared>,
    _thread: JoinHandle<()>,
}

impl WaylandSource {
    fn start() -> Result<Self> {
        let shared = Arc::new(Shared {
            frame: Mutex::new(None),
        });
        let sink = Arc::clone(&shared);
        // Portal negotiation blocks on the user's window pick + permission dialog.
        // Run it (and the PipeWire loop it feeds) on this dedicated thread, never
        // on the caller's, `capture()` is driven from the overlay's draw loop, so
        // blocking here would freeze the whole overlay until the system picker is
        // dismissed. `acquire()` just returns None until the first frame arrives.
        let thread = std::thread::Builder::new()
            .name("hoard-pw".into())
            .spawn(move || {
                let (fd, node_id) = match negotiate_portal() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("hoard-screen: portal negotiation failed: {e}");
                        return;
                    }
                };
                if let Err(e) = pipewire_loop(fd, node_id, sink) {
                    eprintln!("hoard-screen: pipewire loop ended: {e}");
                }
            })
            .map_err(be)?;
        Ok(Self {
            id: "wayland".into(),
            shared,
            _thread: thread,
        })
    }
}

impl Source for WaylandSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn acquire(&mut self) -> Option<Frame> {
        self.shared.frame.lock().ok()?.clone()
    }
}

/// Drive the ScreenCast portal to obtain `(pipewire_fd, node_id)`.
fn negotiate_portal() -> Result<(OwnedFd, u32)> {
    use ashpd::desktop::screencast::{CursorMode, Screencast, SelectSourcesOptions, SourceType};
    use ashpd::desktop::PersistMode;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(be)?;

    rt.block_on(async {
        let proxy = Screencast::new().await.map_err(be)?;
        let session = proxy.create_session(Default::default()).await.map_err(be)?;
        proxy
            .select_sources(
                &session,
                SelectSourcesOptions::default()
                    .set_cursor_mode(CursorMode::Hidden)
                    .set_sources(SourceType::Window | SourceType::Monitor)
                    .set_multiple(false)
                    .set_persist_mode(PersistMode::DoNot),
            )
            .await
            .map_err(be)?;
        let response = proxy
            .start(&session, None, Default::default())
            .await
            .map_err(be)?
            .response()
            .map_err(be)?;
        let stream = response
            .streams()
            .first()
            .ok_or_else(|| CaptureError::Backend("portal returned no streams".into()))?;
        let node_id = stream.pipe_wire_node_id();
        let fd = proxy
            .open_pipe_wire_remote(&session, Default::default())
            .await
            .map_err(be)?;
        Ok((fd, node_id))
    })
}

/// PipeWire client: attach to `node_id` over `fd` and pump frames into `sink`.
fn pipewire_loop(fd: OwnedFd, node_id: u32, sink: Arc<Shared>) -> std::result::Result<(), String> {
    use pipewire as pw;
    use pw::spa;
    use pw::spa::pod::{object, property};
    use pw::{context::ContextBox, main_loop::MainLoopBox, stream::StreamBox};

    pw::init();
    let mainloop = MainLoopBox::new(None).map_err(|e| e.to_string())?;
    let context = ContextBox::new(mainloop.loop_(), None).map_err(|e| e.to_string())?;
    let core = context.connect_fd(fd, None).map_err(|e| e.to_string())?;

    let stream = StreamBox::new(
        &core,
        "hoard-screen-capture",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .map_err(|e| e.to_string())?;

    // Negotiated frame geometry/format, filled in by `param_changed`.
    let format: Rc<RefCell<spa::param::video::VideoInfoRaw>> =
        Rc::new(RefCell::new(spa::param::video::VideoInfoRaw::new()));

    let fmt_cb = Rc::clone(&format);
    let fmt_proc = Rc::clone(&format);
    let sink_cb = Arc::clone(&sink);
    let _listener = stream
        .add_local_listener_with_user_data(())
        .param_changed(move |_stream, _ud, id, param| {
            let Some(param) = param else { return };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }
            let mut info = spa::param::video::VideoInfoRaw::new();
            if info.parse(param).is_ok() {
                *fmt_cb.borrow_mut() = info;
            }
        })
        .process(move |stream, _ud| {
            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buffer.datas_mut();
            let Some(data) = datas.first_mut() else {
                return;
            };
            let chunk = data.chunk();
            let stride = chunk.stride().max(0) as usize;
            let info = fmt_cb_geometry(&fmt_proc);
            let (w, h, vfmt) = info;
            if w == 0 || h == 0 {
                return;
            }
            let Some(bytes) = data.data() else { return };
            if let Some(frame) = to_rgba(bytes, stride, w, h, vfmt) {
                if let Ok(mut slot) = sink_cb.frame.lock() {
                    *slot = Some(frame);
                }
            }
        })
        .register()
        .map_err(|e| e.to_string())?;

    // Ask for common raw formats; the server picks one and reports it back.
    let obj = object! {
        spa::utils::SpaTypes::ObjectParamFormat,
        spa::param::ParamType::EnumFormat,
        property!(spa::param::format::FormatProperties::MediaType, Id, spa::param::format::MediaType::Video),
        property!(spa::param::format::FormatProperties::MediaSubtype, Id, spa::param::format::MediaSubtype::Raw),
        property!(
            spa::param::format::FormatProperties::VideoFormat,
            Choice, Enum, Id,
            spa::param::video::VideoFormat::RGBA,
            spa::param::video::VideoFormat::RGBA,
            spa::param::video::VideoFormat::BGRA,
            spa::param::video::VideoFormat::RGBx,
            spa::param::video::VideoFormat::BGRx
        ),
    };
    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map_err(|e| e.to_string())?
    .0
    .into_inner();
    let mut params = [
        pw::spa::pod::Pod::from_bytes(&values).ok_or_else(|| "invalid format pod".to_string())?
    ];

    stream
        .connect(
            spa::utils::Direction::Input,
            Some(node_id),
            pw::stream::StreamFlags::AUTOCONNECT | pw::stream::StreamFlags::MAP_BUFFERS,
            &mut params,
        )
        .map_err(|e| e.to_string())?;

    mainloop.run();
    Ok(())
}

fn fmt_cb_geometry(
    fmt: &Rc<RefCell<pipewire::spa::param::video::VideoInfoRaw>>,
) -> (usize, usize, pipewire::spa::param::video::VideoFormat) {
    let b = fmt.borrow();
    (
        b.size().width as usize,
        b.size().height as usize,
        b.format(),
    )
}

/// Convert a captured frame (BGRx/RGBx/BGRA/RGBA, possibly padded) to RGBA.
fn to_rgba(
    bytes: &[u8],
    stride: usize,
    w: usize,
    h: usize,
    vfmt: pipewire::spa::param::video::VideoFormat,
) -> Option<Frame> {
    use pipewire::spa::param::video::VideoFormat as F;
    let row = if stride == 0 { w * 4 } else { stride };
    if bytes.len() < row * h {
        return None;
    }
    // (red_index, blue_index) within each 4-byte source pixel.
    let (ri, bi) = match vfmt {
        F::BGRA | F::BGRx => (2, 0),
        _ => (0, 2), // RGBA / RGBx and anything else
    };
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        let src = &bytes[y * row..y * row + w * 4];
        let dst = &mut out[y * w * 4..(y + 1) * w * 4];
        for (s, d) in src.chunks_exact(4).zip(dst.chunks_exact_mut(4)) {
            d[0] = s[ri];
            d[1] = s[1];
            d[2] = s[bi];
            d[3] = 0xff;
        }
    }
    Some(Frame::new(w as u32, h as u32, out))
}
