//! Windows GPU compositor for the overlay (DirectComposition + D3D11).
//!
//! Replaces the old CPU path (WGC frame → CPU readback + BGRA→RGBA swizzle →
//! `UpdateLayeredWindow`) for compat panels. Everything stays on the GPU:
//!
//! * **Capture** ([`GpuCapture`]): a WGC `Direct3D11CaptureFramePool` on the same
//!   D3D11 device the overlay renders with, polled *on the render thread*. The
//!   captured `ID3D11Texture2D` is sampled directly, no `Map`, no readback, no
//!   swizzle (the `B8G8R8A8_UNORM` SRV does the channel order for free). This is
//!   what removes the ~10% CPU the old per-pixel swizzle cost.
//! * **Present** ([`GpuOverlay`]): one `WS_EX_NOREDIRECTIONBITMAP` window per
//!   monitor, fixed at the monitor size (never resized per frame), fed by an
//!   `IDXGISwapChain1` created `ForComposition` and shown through a
//!   DirectComposition visual. Presented with `Present(0)` (see [`GpuOverlay::render`]):
//!   blocking on vblank parked the window thread in the driver and froze
//!   hit-testing, so the run loop paces itself on the message queue instead. The
//!   fixed per-monitor geometry (never touched per frame) is what stops the
//!   interactive-compat hit-test from "losing" the mouse.
//!   The swapchain is `B8G8R8A8` premultiplied, so colour is presented in the
//!   right space (no washed-out/over-bright look).
//!
//! Notes/images keep going through the CPU compositor ([`crate::compositor`]),
//! but their composed RGBA buffer is uploaded to a per-monitor dynamic texture
//! and drawn as one quad on the same swapchain, so the whole overlay is a single
//! DComp window per monitor.

use std::collections::HashMap;

use windows::core::{s, Interface, Result, PCSTR};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::Fxc::{D3DCompile, D3DCOMPILE_OPTIMIZATION_LEVEL3};
use windows::Win32::Graphics::Direct3D::{
    ID3DBlob, D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D_DRIVER_TYPE_HARDWARE,
    D3D_DRIVER_TYPE_WARP,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11BlendState, ID3D11Buffer, ID3D11Device, ID3D11DeviceContext,
    ID3D11InputLayout, ID3D11PixelShader, ID3D11RasterizerState, ID3D11RenderTargetView,
    ID3D11SamplerState, ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
    D3D11_BIND_VERTEX_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE,
    D3D11_BLEND_OP_ADD, D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_NEVER,
    D3D11_CPU_ACCESS_WRITE, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CULL_NONE, D3D11_FILL_SOLID,
    D3D11_FILTER_MIN_MAG_MIP_LINEAR, D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_WRITE_DISCARD, D3D11_RASTERIZER_DESC,
    D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DYNAMIC, D3D11_VIEWPORT,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM,
    DXGI_FORMAT_R32G32_FLOAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIDevice, IDXGIFactory2, IDXGISwapChain1, DXGI_PRESENT, DXGI_SCALING_STRETCH,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

fn map_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

/// One vertex: clip-space position + texture coord. Matches the input layout and
/// the HLSL `VSIn` below.
#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

/// A textured quad to draw this frame: which SRV, where (monitor-local pixels),
/// which sub-rect of the texture (normalized), and whether to force it opaque
/// (captured video) or honour its alpha (notes/images).
pub struct Quad {
    pub srv: ID3D11ShaderResourceView,
    pub dst: (f32, f32, f32, f32), // x, y, w, h in monitor-local pixels
    pub uv: (f32, f32, f32, f32),  // u0, v0, u1, v1
    pub opaque: bool,
}

/// Per-monitor presentation: the overlay window, its composition swapchain, and
/// a reusable dynamic texture for the CPU-composed notes layer.
struct MonGpu {
    _hwnd: HWND,
    w: u32,
    h: u32,
    swapchain: IDXGISwapChain1,
    _target: IDCompositionTarget,
    _visual: IDCompositionVisual,
    notes_tex: Option<ID3D11Texture2D>,
    notes_srv: Option<ID3D11ShaderResourceView>,
}

/// A live WGC capture whose latest frame is held as a GPU texture + SRV on the
/// shared device, polled from the render thread.
struct GpuCapture {
    pool: Direct3D11CaptureFramePool,
    _session: windows::Graphics::Capture::GraphicsCaptureSession,
    d3d: IDirect3DDevice,
    size: SizeInt32,
    srv: Option<ID3D11ShaderResourceView>,
    /// Our own texture the captured frame is copied into. The frame-pool textures
    /// must be returned to the pool (by closing each frame) or it stalls after a
    /// couple of frames, so we never sample them directly; we copy here and keep
    /// this as the stable "latest frame".
    tex: Option<ID3D11Texture2D>,
    src_w: u32,
    src_h: u32,
    hwnd: isize,
    frames: u64,
    got_first: bool,
}

impl GpuCapture {
    fn start(
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        d3d: &IDirect3DDevice,
        hwnd: HWND,
    ) -> Result<Self> {
        let item = item_for_window(hwnd)?;
        let size = item.Size()?;
        // 8-bit B8G8R8A8 capture. (An FP16 scRGB pipeline was tried to dodge the
        // HDR SDR-content brightness boost, but the resulting HDR-aware composition
        // surface gets promoted to a hardware overlay plane that scans ABOVE the
        // hardware cursor → the cursor vanished under the panel. The user runs SDR,
        // where the boost doesn't apply, so 8-bit is both correct and cursor-safe.)
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            d3d,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = pool.CreateCaptureSession(&item)?;
        session.SetIsBorderRequired(false).ok();
        session.SetIsCursorCaptureEnabled(false).ok();
        session.StartCapture()?;
        crate::slog!(
            "capture started hwnd={:#x} item={}x{}",
            hwnd.0 as isize,
            size.Width,
            size.Height
        );
        let mut me = Self {
            pool,
            _session: session,
            d3d: d3d.clone(),
            size,
            srv: None,
            tex: None,
            src_w: size.Width.max(1) as u32,
            src_h: size.Height.max(1) as u32,
            hwnd: hwnd.0 as isize,
            frames: 0,
            got_first: false,
        };
        me.poll(device, context);
        Ok(me)
    }

    /// Drain to the newest available frame, copy it into our own texture, and
    /// **close each frame** so the pool keeps delivering (not closing stalls WGC
    /// after a couple of frames, the Discord/Brave black-panel freeze). No-op
    /// (keeps the last texture) when nothing new arrived. Returns true if a fresh
    /// frame landed.
    fn poll(&mut self, device: &ID3D11Device, context: &ID3D11DeviceContext) -> bool {
        let mut got = false;
        loop {
            let frame = match self.pool.TryGetNextFrame() {
                Ok(f) => f,
                Err(_) => break, // no frame ready
            };
            if let Ok(cs) = frame.ContentSize() {
                if cs.Width != self.size.Width || cs.Height != self.size.Height {
                    self.size = cs;
                    self.tex = None; // size changed: rebuild our copy target + SRV
                    self.srv = None;
                    let _ = self.pool.Recreate(
                        &self.d3d,
                        DirectXPixelFormat::B8G8R8A8UIntNormalized,
                        2,
                        cs,
                    );
                }
            }
            if let Ok(surface) = frame.Surface() {
                if let Ok(access) = surface.cast::<IDirect3DDxgiInterfaceAccess>() {
                    if let Ok(src) = unsafe { access.GetInterface::<ID3D11Texture2D>() } {
                        if unsafe { self.copy_frame(device, context, &src) }.is_ok() {
                            got = true;
                            self.frames += 1;
                            if !self.got_first {
                                self.got_first = true;
                                crate::slog!(
                                    "capture first frame hwnd={:#x} {}x{}",
                                    self.hwnd,
                                    self.src_w,
                                    self.src_h
                                );
                            }
                        }
                    }
                }
            }
            // Return this frame's buffer to the pool. Without it the pool runs dry.
            if let Ok(closable) = frame.cast::<windows::Foundation::IClosable>() {
                let _ = closable.Close();
            }
        }
        got
    }

    /// Copy the pool's frame texture into our own persistent texture (recreating
    /// it + its SRV when the source size changes) so we hold a stable last frame
    /// independent of the pool's recycling.
    unsafe fn copy_frame(
        &mut self,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        src: &ID3D11Texture2D,
    ) -> Result<()> {
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        src.GetDesc(&mut desc);
        let (w, h) = (desc.Width.max(1), desc.Height.max(1));
        if self.tex.is_none() || self.src_w != w || self.src_h != h {
            let own_desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: windows::Win32::Graphics::Direct3D11::D3D11_USAGE_DEFAULT,
                BindFlags: windows::Win32::Graphics::Direct3D11::D3D11_BIND_SHADER_RESOURCE.0
                    as u32,
                ..Default::default()
            };
            let mut tex = None;
            device.CreateTexture2D(&own_desc, None, Some(&mut tex))?;
            let tex = tex.unwrap();
            let srv = create_srv(device, &tex, DXGI_FORMAT_B8G8R8A8_UNORM)?;
            self.tex = Some(tex);
            self.srv = Some(srv);
            self.src_w = w;
            self.src_h = h;
        }
        if let Some(tex) = &self.tex {
            context.CopyResource(tex, src);
        }
        Ok(())
    }
}

pub struct GpuOverlay {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    factory: IDXGIFactory2,
    dcomp: IDCompositionDevice,
    d3d: IDirect3DDevice,
    vs: ID3D11VertexShader,
    layout: ID3D11InputLayout,
    ps_opaque: ID3D11PixelShader,
    ps_alpha: ID3D11PixelShader,
    sampler: ID3D11SamplerState,
    blend: ID3D11BlendState,
    raster: ID3D11RasterizerState,
    vbuf: ID3D11Buffer,
    mons: Vec<MonGpu>,
    captures: HashMap<isize, GpuCapture>,
}

impl GpuOverlay {
    pub fn new() -> std::result::Result<Self, String> {
        unsafe { Self::new_inner() }.map_err(map_err)
    }

    unsafe fn new_inner() -> Result<Self> {
        // WGC's activation factory + frame pool need this thread inside a COM
        // apartment. The capture runs here on the render thread now (not on a
        // separate MTA worker), so initialize it here. Ignore the error: a prior
        // STA init returns RPC_E_CHANGED_MODE, and the free-threaded frame pool
        // and the activation factory work in either apartment.
        let _ = RoInitialize(RO_INIT_MULTITHREADED);

        let (device, context) = create_device()?;
        let dxgi: IDXGIDevice = device.cast()?;
        let adapter = dxgi.GetAdapter()?;
        let factory: IDXGIFactory2 = adapter.GetParent()?;
        let dcomp: IDCompositionDevice = DCompositionCreateDevice(&dxgi)?;
        let d3d_raw = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?;
        let d3d: IDirect3DDevice = d3d_raw.cast()?;

        // Shaders. A pass-through vertex shader (positions are already in clip
        // space) and two pixel shaders: opaque for captured video (alpha forced
        // 1), straight-alpha→premultiplied for notes/images.
        let vs_blob = compile(VS_SRC, s!("vs_4_0"))?;
        let vs_bytes = blob_slice(&vs_blob);
        let mut vs = None;
        device.CreateVertexShader(vs_bytes, None, Some(&mut vs))?;
        let vs = vs.unwrap();

        let elements = [
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("POSITION"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 0,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
            D3D11_INPUT_ELEMENT_DESC {
                SemanticName: s!("TEXCOORD"),
                SemanticIndex: 0,
                Format: DXGI_FORMAT_R32G32_FLOAT,
                InputSlot: 0,
                AlignedByteOffset: 8,
                InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                InstanceDataStepRate: 0,
            },
        ];
        let mut layout = None;
        device.CreateInputLayout(&elements, vs_bytes, Some(&mut layout))?;
        let layout = layout.unwrap();

        let ps_o_blob = compile(PS_OPAQUE_SRC, s!("ps_4_0"))?;
        let mut ps_opaque = None;
        device.CreatePixelShader(blob_slice(&ps_o_blob), None, Some(&mut ps_opaque))?;
        let ps_opaque = ps_opaque.unwrap();

        let ps_a_blob = compile(PS_ALPHA_SRC, s!("ps_4_0"))?;
        let mut ps_alpha = None;
        device.CreatePixelShader(blob_slice(&ps_a_blob), None, Some(&mut ps_alpha))?;
        let ps_alpha = ps_alpha.unwrap();

        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            MinLOD: 0.0,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let mut sampler = None;
        device.CreateSamplerState(&sampler_desc, Some(&mut sampler))?;
        let sampler = sampler.unwrap();

        // Premultiplied "over": the swapchain alpha mode is premultiplied and the
        // pixel shaders emit premultiplied colour, so src=ONE, dst=INV_SRC_ALPHA.
        let mut blend_desc = D3D11_BLEND_DESC::default();
        blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
            BlendEnable: true.into(),
            SrcBlend: D3D11_BLEND_ONE,
            DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
            BlendOp: D3D11_BLEND_OP_ADD,
            SrcBlendAlpha: D3D11_BLEND_ONE,
            DestBlendAlpha: D3D11_BLEND_INV_SRC_ALPHA,
            BlendOpAlpha: D3D11_BLEND_OP_ADD,
            RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
        };
        let mut blend = None;
        device.CreateBlendState(&blend_desc, Some(&mut blend))?;
        let blend = blend.unwrap();

        let raster_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            DepthClipEnable: true.into(),
            ..Default::default()
        };
        let mut raster = None;
        device.CreateRasterizerState(&raster_desc, Some(&mut raster))?;
        let raster = raster.unwrap();

        // Dynamic vertex buffer: 4 verts (one quad), rewritten per draw.
        let vb_desc = D3D11_BUFFER_DESC {
            ByteWidth: (std::mem::size_of::<Vertex>() * 4) as u32,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let mut vbuf = None;
        device.CreateBuffer(&vb_desc, None, Some(&mut vbuf))?;
        let vbuf = vbuf.unwrap();

        Ok(Self {
            device,
            context,
            factory,
            dcomp,
            d3d,
            vs,
            layout,
            ps_opaque,
            ps_alpha,
            sampler,
            blend,
            raster,
            vbuf,
            mons: Vec::new(),
            captures: HashMap::new(),
        })
    }

    /// Bind a monitor's overlay window: create its composition swapchain and the
    /// DComp visual tree that shows it. Call once per monitor at startup.
    pub fn add_monitor(&mut self, hwnd: HWND, w: i32, h: i32) -> std::result::Result<(), String> {
        unsafe { self.add_monitor_inner(hwnd, w.max(1) as u32, h.max(1) as u32) }.map_err(map_err)
    }

    unsafe fn add_monitor_inner(&mut self, hwnd: HWND, w: u32, h: u32) -> Result<()> {
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: w,
            Height: h,
            // 8-bit SDR output. An FP16 scRGB + G10 surface (to dodge the HDR
            // SDR-content brightness boost) made DWM treat this as HDR-aware and
            // promote it to a hardware overlay plane that scanned ABOVE the cursor,
            // hiding the mouse under the panel. The user runs SDR (no boost), so
            // plain B8G8R8A8 is correct and keeps the cursor visible.
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            Flags: 0,
        };
        let swapchain = self
            .factory
            .CreateSwapChainForComposition(&self.device, &desc, None)?;
        let target = self.dcomp.CreateTargetForHwnd(hwnd, true)?;
        let visual = self.dcomp.CreateVisual()?;
        visual.SetContent(&swapchain)?;
        target.SetRoot(&visual)?;
        self.dcomp.Commit()?;
        self.mons.push(MonGpu {
            _hwnd: hwnd,
            w,
            h,
            swapchain,
            _target: target,
            _visual: visual,
            notes_tex: None,
            notes_srv: None,
        });
        Ok(())
    }

    pub fn d3d_device(&self) -> &IDirect3DDevice {
        &self.d3d
    }

    /// Reconcile the live captures with the set of window handles that should be
    /// captured this frame (compat panels in View). Drops captures that are gone,
    /// opens new ones.
    pub fn reconcile_captures(&mut self, wanted: &[isize]) {
        self.captures.retain(|hwnd, _| {
            let keep = wanted.contains(hwnd);
            if !keep {
                crate::slog!("capture dropped hwnd={:#x}", hwnd);
            }
            keep
        });
        for &hwnd in wanted {
            if !self.captures.contains_key(&hwnd) {
                let h = HWND(hwnd as *mut core::ffi::c_void);
                match GpuCapture::start(&self.device, &self.context, &self.d3d, h) {
                    Ok(c) => {
                        self.captures.insert(hwnd, c);
                    }
                    Err(e) => crate::serr!("capture start failed hwnd={hwnd:#x}: {e}"),
                }
            }
        }
    }

    /// Pull the newest frame from every capture. Returns true if any produced a
    /// fresh frame (so the caller can decide to re-present).
    pub fn poll_captures(&mut self) -> bool {
        let mut any = false;
        let device = self.device.clone();
        let context = self.context.clone();
        for c in self.captures.values_mut() {
            any |= c.poll(&device, &context);
        }
        any
    }

    /// Per-capture `(hwnd, total_frames, has_frame)` for diagnostics. A capture
    /// whose `total_frames` stops climbing is frozen (the Discord/Brave black-
    /// panel symptom, DWM stopped composing the parked window).
    pub fn capture_stats(&self) -> Vec<(isize, u64, bool)> {
        self.captures
            .iter()
            .map(|(h, c)| (*h, c.frames, c.srv.is_some()))
            .collect()
    }

    /// The latest SRV + source size for a captured window, if it has a frame.
    pub fn capture_srv(&self, hwnd: isize) -> Option<(ID3D11ShaderResourceView, u32, u32)> {
        let c = self.captures.get(&hwnd)?;
        let srv = c.srv.clone()?;
        Some((srv, c.src_w, c.src_h))
    }

    pub fn monitor_size(&self, idx: usize) -> Option<(u32, u32)> {
        self.mons.get(idx).map(|m| (m.w, m.h))
    }

    /// The SRV of monitor `idx`'s notes texture (valid after a successful
    /// [`upload_notes`](Self::upload_notes)).
    pub fn notes_srv(&self, idx: usize) -> Option<ID3D11ShaderResourceView> {
        self.mons.get(idx).and_then(|m| m.notes_srv.clone())
    }

    /// Upload the CPU-composed notes/images RGBA buffer (monitor-sized) to the
    /// per-monitor dynamic texture and return its SRV as a full-frame quad source.
    pub fn upload_notes(
        &mut self,
        idx: usize,
        rgba: &[u8],
    ) -> std::result::Result<Option<ID3D11ShaderResourceView>, String> {
        unsafe { self.upload_notes_inner(idx, rgba) }.map_err(map_err)
    }

    unsafe fn upload_notes_inner(
        &mut self,
        idx: usize,
        rgba: &[u8],
    ) -> Result<Option<ID3D11ShaderResourceView>> {
        let (w, h) = match self.mons.get(idx) {
            Some(m) => (m.w, m.h),
            None => return Ok(None),
        };
        if self.mons[idx].notes_tex.is_none() {
            let desc = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DYNAMIC,
                BindFlags: windows::Win32::Graphics::Direct3D11::D3D11_BIND_SHADER_RESOURCE.0
                    as u32,
                CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
                ..Default::default()
            };
            let mut tex = None;
            self.device.CreateTexture2D(&desc, None, Some(&mut tex))?;
            let tex = tex.unwrap();
            let srv = create_srv(&self.device, &tex, DXGI_FORMAT_R8G8B8A8_UNORM)?;
            self.mons[idx].notes_tex = Some(tex);
            self.mons[idx].notes_srv = Some(srv);
        }
        let tex = self.mons[idx].notes_tex.clone().unwrap();
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        self.context
            .Map(&tex, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped))?;
        let row_bytes = (w as usize) * 4;
        let dst = mapped.pData as *mut u8;
        let pitch = mapped.RowPitch as usize;
        for y in 0..(h as usize) {
            let src = rgba.as_ptr().add(y * row_bytes);
            std::ptr::copy_nonoverlapping(src, dst.add(y * pitch), row_bytes);
        }
        self.context.Unmap(&tex, 0);
        Ok(self.mons[idx].notes_srv.clone())
    }

    /// Clear monitor `idx`'s swapchain to transparent, draw the quads back to
    /// front, and present at vblank.
    pub fn render(&mut self, idx: usize, quads: &[Quad]) -> std::result::Result<(), String> {
        unsafe { self.render_inner(idx, quads) }.map_err(map_err)
    }

    unsafe fn render_inner(&mut self, idx: usize, quads: &[Quad]) -> Result<()> {
        let (w, h) = match self.mons.get(idx) {
            Some(m) => (m.w as f32, m.h as f32),
            None => return Ok(()),
        };
        let swapchain = self.mons[idx].swapchain.clone();
        let back: ID3D11Texture2D = swapchain.GetBuffer(0)?;
        let mut rtv: Option<ID3D11RenderTargetView> = None;
        self.device
            .CreateRenderTargetView(&back, None, Some(&mut rtv))?;
        let rtv = rtv.unwrap();

        let ctx = &self.context;
        ctx.OMSetRenderTargets(Some(&[Some(rtv.clone())]), None);
        let vp = D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: w,
            Height: h,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        };
        ctx.RSSetViewports(Some(&[vp]));
        ctx.RSSetState(&self.raster);
        let clear = [0.0f32, 0.0, 0.0, 0.0];
        ctx.ClearRenderTargetView(&rtv, &clear);

        ctx.IASetInputLayout(&self.layout);
        ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
        let stride = std::mem::size_of::<Vertex>() as u32;
        let offset = 0u32;
        ctx.IASetVertexBuffers(
            0,
            1,
            Some(&Some(self.vbuf.clone())),
            Some(&stride),
            Some(&offset),
        );
        ctx.VSSetShader(&self.vs, None);
        ctx.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
        let factor = [0.0f32; 4];
        ctx.OMSetBlendState(&self.blend, Some(&factor), 0xffffffff);

        for q in quads {
            // Pixels → clip space (top-left origin to NDC).
            let (x, y, qw, qh) = q.dst;
            let l = x / w * 2.0 - 1.0;
            let r = (x + qw) / w * 2.0 - 1.0;
            let t = 1.0 - y / h * 2.0;
            let b = 1.0 - (y + qh) / h * 2.0;
            let (u0, v0, u1, v1) = q.uv;
            let verts = [
                Vertex {
                    pos: [l, t],
                    uv: [u0, v0],
                },
                Vertex {
                    pos: [r, t],
                    uv: [u1, v0],
                },
                Vertex {
                    pos: [l, b],
                    uv: [u0, v1],
                },
                Vertex {
                    pos: [r, b],
                    uv: [u1, v1],
                },
            ];
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            ctx.Map(&self.vbuf, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped))?;
            std::ptr::copy_nonoverlapping(
                verts.as_ptr() as *const u8,
                mapped.pData as *mut u8,
                std::mem::size_of::<Vertex>() * 4,
            );
            ctx.Unmap(&self.vbuf, 0);

            let ps = if q.opaque {
                &self.ps_opaque
            } else {
                &self.ps_alpha
            };
            ctx.PSSetShader(ps, None);
            ctx.PSSetShaderResources(0, Some(&[Some(q.srv.clone())]));
            ctx.Draw(4, 0);
        }

        // Unbind SRVs so a captured texture isn't left bound as input next frame.
        ctx.PSSetShaderResources(0, Some(&[None]));
        ctx.OMSetRenderTargets(Some(&[None]), None);

        // `Present(0)`, do NOT block the thread on vblank. `Present(1)` parked the
        // window thread in the driver for up to a frame, during which it couldn't
        // answer `WM_NCHITTEST`; with a full-screen topmost overlay that froze the
        // mouse across the whole desktop. The loop paces itself with a
        // message-aware wait instead (see `run_inner`), so the thread is always
        // free to hit-test.
        let _ = swapchain.Present(0, DXGI_PRESENT(0));
        Ok(())
    }
}

unsafe fn create_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device = None;
        let mut context = None;
        let hr = D3D11CreateDevice(
            None,
            driver,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        );
        if hr.is_ok() {
            let kind = if driver == D3D_DRIVER_TYPE_HARDWARE {
                "hardware"
            } else {
                "WARP"
            };
            crate::slog!("gpu device created ({kind})");
            return Ok((device.unwrap(), context.unwrap()));
        }
    }
    crate::serr!("gpu device creation failed (no hardware, no WARP)");
    Err(windows::core::Error::from_win32())
}

unsafe fn create_srv(
    device: &ID3D11Device,
    tex: &ID3D11Texture2D,
    format: DXGI_FORMAT,
) -> Result<ID3D11ShaderResourceView> {
    use windows::Win32::Graphics::Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D;
    use windows::Win32::Graphics::Direct3D11::{
        D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_SRV,
    };
    let desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
        Format: format,
        ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
        Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
            Texture2D: D3D11_TEX2D_SRV {
                MostDetailedMip: 0,
                MipLevels: 1,
            },
        },
    };
    let mut srv = None;
    device.CreateShaderResourceView(tex, Some(&desc), Some(&mut srv))?;
    Ok(srv.unwrap())
}

fn item_for_window(hwnd: HWND) -> Result<GraphicsCaptureItem> {
    let interop: IGraphicsCaptureItemInterop =
        windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
    unsafe { interop.CreateForWindow(hwnd) }
}

unsafe fn compile(src: &str, target: windows::core::PCSTR) -> Result<ID3DBlob> {
    let mut code: Option<ID3DBlob> = None;
    let mut errors: Option<ID3DBlob> = None;
    let res = D3DCompile(
        src.as_ptr() as *const core::ffi::c_void,
        src.len(),
        PCSTR::null(),
        None,
        None,
        s!("main"),
        target,
        D3DCOMPILE_OPTIMIZATION_LEVEL3,
        0,
        &mut code,
        Some(&mut errors),
    );
    if let Err(e) = res {
        if let Some(err) = errors {
            let msg = std::slice::from_raw_parts(
                err.GetBufferPointer() as *const u8,
                err.GetBufferSize(),
            );
            eprintln!(
                "hoard-screen: shader compile error: {}",
                String::from_utf8_lossy(msg)
            );
        }
        return Err(e);
    }
    Ok(code.unwrap())
}

unsafe fn blob_slice(blob: &ID3DBlob) -> &[u8] {
    std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize())
}

const VS_SRC: &str = r#"
struct VSIn  { float2 pos : POSITION; float2 uv : TEXCOORD; };
struct VSOut { float4 pos : SV_POSITION; float2 uv : TEXCOORD; };
VSOut main(VSIn i) {
    VSOut o;
    o.pos = float4(i.pos, 0.0, 1.0);
    o.uv  = i.uv;
    return o;
}
"#;

const PS_OPAQUE_SRC: &str = r#"
Texture2D    tex : register(t0);
SamplerState smp : register(s0);
// Capture and swapchain are both 8-bit B8G8R8A8 (no sRGB views): pass the
// captured content through byte-for-byte. Forced opaque so the video covers the
// game beneath it.
float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET {
    float3 c = tex.Sample(smp, uv).rgb;
    return float4(c, 1.0);
}
"#;

const PS_ALPHA_SRC: &str = r#"
Texture2D    tex : register(t0);
SamplerState smp : register(s0);
// Notes/images: straight premultiply against the 8-bit swapchain.
float4 main(float4 pos : SV_POSITION, float2 uv : TEXCOORD) : SV_TARGET {
    float4 c = tex.Sample(smp, uv);
    return float4(c.rgb * c.a, c.a);
}
"#;
