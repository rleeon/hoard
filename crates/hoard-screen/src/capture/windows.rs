//! Windows window capture via **Windows.Graphics.Capture** (WGC), the modern,
//! OBS-grade path: a `GraphicsCaptureItem` built from an HWND feeds a
//! free-threaded `Direct3D11CaptureFramePool`; each captured `IDirect3DSurface`
//! is copied to a CPU-readable staging texture and swizzled BGRA -> RGBA.
//! Unlike the old BitBlt path it handles GPU-rendered and occluded windows and
//! composites correctly over fullscreen games.
//!
//! Capture runs on its own thread per source: it polls `TryGetNextFrame` and
//! stows the latest frame behind a mutex, so `acquire()` is a cheap clone and
//! the overlay never blocks on the GPU. DRM-protected windows (Netflix &c.)
//! capture black by design, see the module note in `mod.rs`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use windows::core::Interface;
use windows::core::PWSTR;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowDisplayAffinity, GetWindowLongW, GetWindowRect, GetWindowTextLengthW,
    GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, GWL_EXSTYLE, WDA_EXCLUDEFROMCAPTURE,
    WDA_MONITOR, WS_EX_TOOLWINDOW,
};

use super::{CaptureBackend, CaptureError, Result, WindowId, WindowInfo};
use crate::source::{Frame, Source};

fn backend_err<E: std::fmt::Display>(e: E) -> CaptureError {
    CaptureError::Backend(e.to_string())
}

pub struct WindowsCapture;

impl WindowsCapture {
    pub fn new() -> Self {
        Self
    }
}

impl CaptureBackend for WindowsCapture {
    fn name(&self) -> &'static str {
        "windows/graphics-capture"
    }

    fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        let mut out: Vec<WindowInfo> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(enum_proc), LPARAM(&mut out as *mut _ as isize));
        }
        Ok(out)
    }

    fn capture(&self, id: &WindowId) -> Result<Box<dyn Source>> {
        let hwnd = parse_hwnd(id)?;
        WgcSource::start(hwnd, id.clone()).map(|s| Box::new(s) as Box<dyn Source>)
    }
}

/// `EnumWindows` callback: collect visible, titled, top-level app windows.
unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam.0 as *mut Vec<WindowInfo>);

    if !IsWindowVisible(hwnd).as_bool() {
        return TRUE;
    }
    // Skip DWM-cloaked windows: UWP/background ghosts that report "visible" but
    // aren't actually on screen, "Windows Input Experience", the suspended
    // "Descargas - Explorador de archivos" the user never opened, etc.
    if is_cloaked(hwnd) {
        return TRUE;
    }
    // Skip tool windows (tray helpers, etc.).
    let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    if ex & WS_EX_TOOLWINDOW.0 != 0 {
        return TRUE;
    }
    let len = GetWindowTextLengthW(hwnd);
    if len <= 0 {
        return TRUE;
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = GetWindowTextW(hwnd, &mut buf);
    if n <= 0 {
        return TRUE;
    }
    let title = String::from_utf16_lossy(&buf[..n as usize]);
    // Friendly app name from the owning process's executable (Brave, Discord,
    // …), and skip our own windows so Hoard doesn't list itself.
    let app = window_app_name(hwnd).unwrap_or_default();
    let low = app.to_ascii_lowercase();
    if low == "hoard" || low == "hoard-desktop" {
        return TRUE;
    }
    out.push(WindowInfo {
        id: format!("0x{:x}", hwnd.0 as isize),
        title,
        app,
        protected: is_capture_protected(hwnd),
    });
    TRUE
}

/// True when DWM reports the window as cloaked (hidden by the compositor even
/// though `IsWindowVisible` is true). Filters out UWP/background ghost windows.
unsafe fn is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked: u32 = 0;
    let ok = DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED,
        &mut cloaked as *mut _ as *mut core::ffi::c_void,
        std::mem::size_of::<u32>() as u32,
    )
    .is_ok();
    ok && cloaked != 0
}

/// Friendly app name from the window's owning process, the executable stem with
/// a capitalized first letter ("brave.exe" -> "Brave"). `None` if we can't open
/// the process (cross-integrity-level, already gone, …).
unsafe fn window_app_name(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return None;
    }
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
    let mut buf = vec![0u16; 512];
    let mut len = buf.len() as u32;
    let res = QueryFullProcessImageNameW(
        handle,
        PROCESS_NAME_WIN32,
        PWSTR(buf.as_mut_ptr()),
        &mut len,
    );
    let _ = CloseHandle(handle);
    res.ok()?;
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    let base = path.rsplit(['\\', '/']).next().unwrap_or(&path);
    let stem = base
        .strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .unwrap_or(base);
    if stem.is_empty() {
        return None;
    }
    let mut chars = stem.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().collect::<String>() + chars.as_str())
}

/// True when the window has opted out of screen capture via display affinity
/// (`SetWindowDisplayAffinity`): `WDA_MONITOR` (visible on the monitor only, a
/// capture sees black) or `WDA_EXCLUDEFROMCAPTURE` (omitted from captures).
/// Purely informational, so the editor can warn the user; we never try to defeat
/// it. A failed query (e.g. cross-integrity-level window) is treated as not
/// protected. Note: this is *not* the only black-capture cause, HDCP / the
/// protected media path can also yield black with `WDA_NONE`, so a `false` here
/// is not a guarantee the window captures cleanly.
unsafe fn is_capture_protected(hwnd: HWND) -> bool {
    let mut affinity: u32 = 0;
    if GetWindowDisplayAffinity(hwnd, &mut affinity).is_err() {
        return false;
    }
    affinity == WDA_MONITOR.0 || affinity == WDA_EXCLUDEFROMCAPTURE.0
}

fn parse_hwnd(id: &WindowId) -> Result<HWND> {
    let s = id.trim();
    let val = if let Some(hex) = s.strip_prefix("0x") {
        isize::from_str_radix(hex, 16)
    } else {
        s.parse::<isize>()
    };
    val.map(|v| HWND(v as *mut core::ffi::c_void))
        .map_err(|_| CaptureError::Backend(format!("bad window id: {id}")))
}

/// A live WGC source: a background thread pumps frames into `latest`.
struct WgcSource {
    id: String,
    latest: Arc<Mutex<Option<Frame>>>,
    running: Arc<AtomicBool>,
    /// Bumped by the capture thread each time a new frame lands. `acquire`
    /// compares it against `last_seen` so it returns `None` (and the overlay
    /// skips a recompose + `UpdateLayeredWindow`) when nothing changed.
    generation: Arc<AtomicU64>,
    last_seen: u64,
}

impl WgcSource {
    fn start(hwnd: HWND, id: String) -> Result<Self> {
        // Validate the window has a usable size up front.
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect).map_err(backend_err)? };
        if rect.right - rect.left <= 0 || rect.bottom - rect.top <= 0 {
            return Err(CaptureError::NotFound(id));
        }

        let latest: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
        let running = Arc::new(AtomicBool::new(true));
        let generation = Arc::new(AtomicU64::new(0));
        let hwnd_val = hwnd.0 as isize;
        let latest_t = Arc::clone(&latest);
        let running_t = Arc::clone(&running);
        let generation_t = Arc::clone(&generation);

        std::thread::spawn(move || {
            // WGC is WinRT: this thread must initialise an MTA apartment before
            // the capture factory/interop, or `item_for_window` fails with
            // RO_E_UNINITIALIZED. (The overlay places real windows on Windows and
            // no longer drives this path, but keep it correct in case the capture
            // backend is reused for an embedded-capture mode.)
            let inited = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };
            let hwnd = HWND(hwnd_val as *mut core::ffi::c_void);
            if let Err(e) = pump(hwnd, &latest_t, &running_t, &generation_t) {
                eprintln!("hoard-screen: wgc capture stopped: {e}");
            }
            if inited {
                unsafe { RoUninitialize() };
            }
        });

        Ok(Self {
            id,
            latest,
            running,
            generation,
            last_seen: 0,
        })
    }
}

impl Drop for WgcSource {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Source for WgcSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn acquire(&mut self) -> Option<Frame> {
        // Nothing new since last time → tell the overlay to keep the last frame
        // (no recompose/present). `Acquire` pairs with the capture thread's
        // `Release` bump so the frame write is visible before we read it.
        let g = self.generation.load(Ordering::Acquire);
        if g == self.last_seen {
            return None;
        }
        self.last_seen = g;
        self.latest.lock().ok()?.clone()
    }
}

/// Build the D3D11 device + WGC session and loop reading frames until `running`
/// flips false. Each frame is copied off the GPU into an RGBA [`Frame`].
fn pump(
    hwnd: HWND,
    latest: &Arc<Mutex<Option<Frame>>>,
    running: &Arc<AtomicBool>,
    generation: &Arc<AtomicU64>,
) -> windows::core::Result<()> {
    let (device, context) = create_d3d_device()?;
    let dxgi: IDXGIDevice = device.cast()?;
    let direct3d_device = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? };
    let direct3d_device: IDirect3DDevice = direct3d_device.cast()?;

    let item = item_for_window(hwnd)?;
    let mut size = item.Size()?;

    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &direct3d_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        size,
    )?;
    let session = frame_pool.CreateCaptureSession(&item)?;
    // No yellow capture border around the window and no cursor baked into the
    // frame. Both setters are Win11+; ignore the error on older builds (the
    // capture still works, just with the OS default border).
    session.SetIsBorderRequired(false).ok();
    session.SetIsCursorCaptureEnabled(false).ok();
    session.StartCapture()?;

    // Reused CPU-readable staging texture. Allocating one per frame (the old
    // path) churned the allocator and the driver every tick; we keep it and
    // only recreate it when the captured size changes.
    let mut staging: Option<ID3D11Texture2D> = None;

    while running.load(Ordering::Relaxed) {
        let frame = match frame_pool.TryGetNextFrame() {
            Ok(f) => f,
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(8));
                continue;
            }
        };

        let content_size = frame.ContentSize()?;
        if content_size.Width != size.Width || content_size.Height != size.Height {
            // Window resized: recreate the pool at the new size.
            size = content_size;
            frame_pool.Recreate(
                &direct3d_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                size,
            )?;
        }

        if let Some(out) = copy_frame(&device, &context, &frame, &mut staging)? {
            if let Ok(mut guard) = latest.lock() {
                *guard = Some(out);
                // Publish after the frame is in place; `acquire` reads this with
                // `Acquire` ordering to know a fresh frame is ready.
                generation.fetch_add(1, Ordering::Release);
            }
        }
        drop(frame);
    }

    session.Close().ok();
    frame_pool.Close().ok();
    Ok(())
}

fn create_d3d_device() -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext)> {
    for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        let hr = unsafe {
            D3D11CreateDevice(
                None,
                driver,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        };
        if hr.is_ok() {
            return Ok((device.unwrap(), context.unwrap()));
        }
    }
    Err(windows::core::Error::from_win32())
}

fn item_for_window(hwnd: HWND) -> windows::core::Result<GraphicsCaptureItem> {
    let interop: IGraphicsCaptureItemInterop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForWindow(hwnd) }
}

/// Copy one captured frame off the GPU into an RGBA [`Frame`].
fn copy_frame(
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
    frame: &Direct3D11CaptureFrame,
    staging_cache: &mut Option<ID3D11Texture2D>,
) -> windows::core::Result<Option<Frame>> {
    let surface = frame.Surface()?;
    let access: IDirect3DDxgiInterfaceAccess = surface.cast()?;
    let source: ID3D11Texture2D = unsafe { access.GetInterface()? };

    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { source.GetDesc(&mut desc) };

    // Reuse the cached CPU-readable staging texture unless the captured size
    // changed (window resize), in which case drop it and make a new one.
    let needs_new = match staging_cache {
        Some(tex) => {
            let mut sdesc = D3D11_TEXTURE2D_DESC::default();
            unsafe { tex.GetDesc(&mut sdesc) };
            sdesc.Width != desc.Width || sdesc.Height != desc.Height || sdesc.Format != desc.Format
        }
        None => true,
    };
    if needs_new {
        let mut staging_desc = desc;
        staging_desc.Usage = D3D11_USAGE_STAGING;
        staging_desc.BindFlags = 0;
        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        staging_desc.MiscFlags = 0;
        let mut new_staging: Option<ID3D11Texture2D> = None;
        unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut new_staging))? };
        *staging_cache = new_staging;
    }
    let staging = staging_cache.as_ref().unwrap();

    unsafe { context.CopyResource(staging, &source) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))? };

    let (w, h) = (desc.Width as usize, desc.Height as usize);
    let mut rgba = vec![0u8; w * h * 4];
    unsafe {
        let src = mapped.pData as *const u8;
        let pitch = mapped.RowPitch as usize;
        for y in 0..h {
            let row = src.add(y * pitch);
            for x in 0..w {
                let p = row.add(x * 4);
                let b = *p;
                let g = *p.add(1);
                let r = *p.add(2);
                let i = (y * w + x) * 4;
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 0xff;
            }
        }
        context.Unmap(staging, 0);
    }

    Ok(Some(Frame::new(w as u32, h as u32, rgba)))
}
