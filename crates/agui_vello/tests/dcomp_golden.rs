#![cfg(windows)]

use std::ffi::c_void;
use std::sync::Arc;
use std::time::{Duration, Instant};

use agui::WindowRenderer;
use agui::dcomp::Dcomp;
use agui::geometry::{Rect, Size};
use agui::paint::Canvas;
use agui::paint::compositing::{
    CompositedFrame, Compositor, ContainerLayer, LayerHandle, OffsetLayer, OpacityLayer,
    PictureLayer, SurfaceTransformLayer,
};
use agui::paint::peniko::{Color, Fill, kurbo::Affine};
use agui::wgpu::Backends;
use agui_test::golden::{Image, render_to_image};
use agui_vello::VelloRenderer;

use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::core::{Interface, factory};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    platform::windows::EventLoopBuilderExtWindows,
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId},
};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 300;

const ORANGE: Color = Color::from_rgb8(255, 138, 0);
const BACKGROUND: Color = Color::from_rgb8(30, 30, 30);

/// A frame placing a half-opacity orange box, behind which a surface sits so the opacity wraps a
/// `Surface` node (the case the bug lived in) rather than baking into a raster. The box is centered so
/// the window's center pixel lands on it, over a window-filling background so the composed window has
/// no transparent gaps that would capture the desktop behind it.
fn opacity_over_surface_frame() -> CompositedFrame {
    let background = PictureLayer::new(Canvas::record(|canvas| {
        let brush = canvas.brush(BACKGROUND);
        canvas.fill(
            Fill::NonZero,
            brush,
            &Rect::from(Size::new(f32::from(WIDTH as u16), f32::from(HEIGHT as u16))),
        );
    }));

    let box_picture = PictureLayer::new(Canvas::record(|canvas| {
        let brush = canvas.brush(ORANGE);
        canvas.fill(Fill::NonZero, brush, &Rect::from(Size::new(200.0, 150.0)));
    }));

    let mut surface = SurfaceTransformLayer::new(Affine::translate((100.0, 75.0)));
    surface.append(LayerHandle::new(box_picture).into());

    let mut opacity = OpacityLayer::new(0.5);
    opacity.append(LayerHandle::new(surface).into());

    let mut root = OffsetLayer::new();
    root.append(LayerHandle::new(background).into());
    root.append(LayerHandle::new(opacity).into());

    Compositor::compose(&LayerHandle::new(root))
}

/// The captured composed output of a window: its physical size, the scale it was presented at, and its
/// pixels.
struct Capture {
    width: u32,
    height: u32,
    scale: f64,
    bgra: Vec<u8>,
}

impl Capture {
    fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let i = ((y * self.width + x) * 4) as usize;
        (self.bgra[i + 2], self.bgra[i + 1], self.bgra[i])
    }

    /// The capture as an RGBA [`Image`], for comparison against a headless render.
    fn to_image(&self) -> Image {
        let mut rgba = vec![0u8; self.bgra.len()];
        for (dst, src) in rgba.chunks_exact_mut(4).zip(self.bgra.chunks_exact(4)) {
            dst[0] = src[2];
            dst[1] = src[1];
            dst[2] = src[0];
            dst[3] = src[3];
        }
        Image {
            width: self.width,
            height: self.height,
            rgba,
        }
    }
}

/// Presents `frame` to a borderless window through `Dcomp` at the window's scale factor,
/// then captures the composed output. The window has no border so the captured pixels are exactly the
/// presented content, comparable against a headless render of the same size.
fn present_and_capture(frame: CompositedFrame) -> Capture {
    struct App {
        window: Option<Arc<Window>>,
        renderer: Dcomp<VelloRenderer>,
        frame: CompositedFrame,
        scale: f64,
        result: Option<(u32, u32, Vec<u8>)>,
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.window.is_some() {
                return;
            }

            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("agui · dcomp_golden")
                            .with_decorations(false)
                            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT)),
                    )
                    .unwrap(),
            );

            self.scale = window.scale_factor();
            let size = window.inner_size();
            self.renderer
                .attach(window.clone(), size.width, size.height);
            self.renderer.present(&self.frame, self.scale);

            self.window = Some(window);
            event_loop.set_control_flow(ControlFlow::Poll);
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            if self.result.is_some() {
                return;
            }
            let Some(window) = &self.window else {
                return;
            };

            // Present again and let DWM compose before capturing.
            self.renderer.present(&self.frame, self.scale);
            std::thread::sleep(Duration::from_millis(120));

            self.result = Some(unsafe { capture_window(hwnd_of(window)) });
            event_loop.exit();
        }

        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: winit::event::WindowEvent) {
        }
    }

    let event_loop = EventLoop::builder()
        .with_any_thread(true)
        .build()
        .expect("event loop");

    let mut app = App {
        window: None,
        renderer: Dcomp::new(VelloRenderer::with_backends(Backends::DX12).expect("a DX12 adapter")),
        frame,
        scale: 1.0,
        result: None,
    };
    event_loop.run_app(&mut app).expect("run app");

    let (width, height, bgra) = app.result.expect("a capture");
    Capture {
        width,
        height,
        scale: app.scale,
        bgra,
    }
}

/// Renders `frame` headlessly through Vello at `width` by `height` physical pixels, scaling its logical
/// content by `scale` the way the windowed renderers do, so the result is comparable to a capture.
fn render_vello(renderer: &mut VelloRenderer, frame: &CompositedFrame, capture: &Capture) -> Image {
    render_to_image(
        renderer,
        frame,
        capture.width,
        capture.height,
        capture.scale,
    )
}

/// Asserts the DirectComposition renderer (via `capture`) and the headless Vello renderer compose
/// `frame` to mostly the same image: at most `max_differing` of the pixels may differ by more than 24
/// in any channel. On a mismatch it writes `<label>.vello.png`, `<label>.dcomp.png`, and
/// `<label>.diff.png` for inspection.
///
/// `frame` must not contain an [`External`](agui::paint::compositing::CompositedNode::External)
/// node, which Vello cannot rasterize.
fn assert_renderers_match(
    frame: &CompositedFrame,
    capture: &Capture,
    label: &str,
    max_differing: f32,
) {
    let mut renderer = VelloRenderer::default();

    let dcomp = capture.to_image();
    let vello = render_vello(&mut renderer, frame, capture);

    let differing = vello.diff_pixels(&dcomp, 24);
    let total = (capture.width * capture.height) as f32;
    let fraction = differing as f32 / total;

    if fraction > max_differing {
        vello.save_png(format!("{label}.vello.png")).ok();
        dcomp.save_png(format!("{label}.dcomp.png")).ok();
        if let Some(diff) = vello.diff_image(&dcomp, 24) {
            diff.save_png(format!("{label}.diff.png")).ok();
        }
        panic!(
            "dcomp and vello differ in {:.1}% of pixels (limit {:.1}%); wrote {label}.vello.png, \
             {label}.dcomp.png, {label}.diff.png",
            fraction * 100.0,
            max_differing * 100.0,
        );
    }
}

/// Captures `hwnd`'s composed output through Windows.Graphics.Capture, returning its width, height, and
/// BGRA pixels.
unsafe fn capture_window(hwnd: HWND) -> (u32, u32, Vec<u8>) {
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_1]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .expect("d3d11 device");
    let device = device.unwrap();
    let context = context.unwrap();

    let dxgi: IDXGIDevice = device.cast().expect("dxgi device");
    let rt_device: IDirect3DDevice = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
        .expect("winrt device")
        .cast()
        .expect("winrt device cast");

    let interop: IGraphicsCaptureItemInterop =
        factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().expect("capture interop");
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd).expect("capture item") };
    let size = item.Size().expect("item size");

    let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &rt_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        size,
    )
    .expect("frame pool");
    let session = pool.CreateCaptureSession(&item).expect("capture session");
    session.StartCapture().expect("start capture");

    // Drain to the most recent frame rather than the first: the first frame the pool produces can be
    // the window before our content composed, so keep grabbing for a settle window and use the latest.
    let frame = {
        let mut latest = None;
        let deadline = Instant::now() + Duration::from_millis(400);
        while Instant::now() < deadline {
            match pool.TryGetNextFrame() {
                Ok(next) => latest = Some(next),
                Err(_) => std::thread::sleep(Duration::from_millis(8)),
            }
        }
        latest.expect("a captured frame")
    };

    let surface = frame.Surface().expect("frame surface");
    let access: IDirect3DDxgiInterfaceAccess = surface.cast().expect("surface access");
    let texture: ID3D11Texture2D = unsafe { access.GetInterface() }.expect("texture");

    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut desc) };

    let mut staging_desc = desc;
    staging_desc.Usage = D3D11_USAGE_STAGING;
    staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    staging_desc.BindFlags = 0;
    staging_desc.MiscFlags = 0;

    let mut staging: Option<ID3D11Texture2D> = None;
    unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging)) }
        .expect("staging texture");
    let staging = staging.unwrap();

    unsafe { context.CopyResource(&staging, &texture) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }.expect("map staging");

    let (width, height) = (desc.Width, desc.Height);
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        unsafe {
            let src = (mapped.pData as *const u8).add((y * mapped.RowPitch) as usize);
            let dst = pixels.as_mut_ptr().add((y * width * 4) as usize);
            std::ptr::copy_nonoverlapping(src, dst, (width * 4) as usize);
        }
    }

    unsafe { context.Unmap(&staging, 0) };
    session.Close().ok();
    pool.Close().ok();

    (width, height, pixels)
}

fn hwnd_of(window: &Window) -> HWND {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut c_void),
        other => panic!("expected a Win32 window handle, got {other:?}"),
    }
}

/// An `Opacity(0.5)` over a surface-backed orange box composes to the same image under DComp and Vello:
/// the box is half-transparent (the regression guard for the v3-visual opacity fix), not full orange,
/// and the whole composed window matches a headless rasterize of the frame. One capture serves both
/// checks, so the test opens a single window: two window-opening tests would race the event loop and
/// GPU under cargo's parallel runner.
#[test]
#[ignore = "opens a window and captures the screen; needs a live desktop and GPU"]
fn dcomp_matches_vello_for_opacity_over_a_surface() {
    let frame = opacity_over_surface_frame();
    let capture = present_and_capture(frame.clone());

    // 0.5 * orange (255,138,0) + 0.5 * background (30,30,30) ≈ (142, 84, 15) at the box center.
    let center = capture.pixel(capture.width / 2, capture.height / 2);
    let near = |a: u8, b: u8| a.abs_diff(b) <= 12;
    assert!(
        near(center.0, 142) && near(center.1, 84) && near(center.2, 15),
        "box center at half opacity: got {center:?}, expected ~(142, 84, 15)"
    );

    assert_renderers_match(&frame, &capture, "opacity_over_surface", 0.04);
}
