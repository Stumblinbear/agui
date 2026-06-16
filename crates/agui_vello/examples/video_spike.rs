//! Windows backend step: play a video through the Media Foundation **Media Engine** into a
//! composition swapchain that DirectComposition composes onto the window. This is the `External`
//! surface path end to end (decode → swapchain → system visual), proven standalone before it is wired
//! into the renderer. Windows-only.
//!
//! Run with `cargo run -p agui_vello --example video_spike -- "C:\\path\\to\\video.mp4"`.

#[cfg(not(windows))]
fn main() {
    eprintln!("video_spike is Windows-only");
}

#[cfg(windows)]
fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: video_spike <path-to-video>");
    backend::run(path);
}

#[cfg(windows)]
mod backend {
    #![allow(unsafe_op_in_unsafe_fn, clippy::cast_possible_truncation)]

    use std::cell::Cell;
    use std::ffi::c_void;
    use std::sync::Arc;

    use windows::Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_1},
            Direct3D11::{
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
                D3D11_SDK_VERSION, D3D11CreateDevice, ID3D11Device, ID3D11Texture2D,
            },
            DirectComposition::{
                DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget,
                IDCompositionVisual,
            },
            Dxgi::{
                Common::{
                    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory2, DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH,
                DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
                DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIFactory2, IDXGISwapChain3,
            },
        },
        Media::MediaFoundation::{
            CLSID_MFMediaEngineClassFactory, IMFAttributes, IMFDXGIDeviceManager, IMFMediaEngine,
            IMFMediaEngineClassFactory, IMFMediaEngineNotify, IMFMediaEngineNotify_Impl,
            MF_API_VERSION, MF_MEDIA_ENGINE_CALLBACK, MF_MEDIA_ENGINE_DXGI_MANAGER,
            MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT, MF_SDK_VERSION, MFCreateAttributes,
            MFCreateDXGIDeviceManager, MFSTARTUP_NOSOCKET, MFStartup,
        },
        System::Com::{
            CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
        },
    };
    use windows::core::{BSTR, Interface, implement};
    use winit::{
        application::ApplicationHandler,
        dpi::LogicalSize,
        event::WindowEvent,
        event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
        window::{Window, WindowId},
    };

    const WIDTH: u32 = 960;
    const HEIGHT: u32 = 540;

    /// The Media Engine requires a notify sink for its events; this one only keeps the engine happy.
    #[implement(IMFMediaEngineNotify)]
    struct Notify;

    impl IMFMediaEngineNotify_Impl for Notify_Impl {
        fn EventNotify(
            &self,
            _event: u32,
            _param1: usize,
            _param2: u32,
        ) -> windows::core::Result<()> {
            Ok(())
        }
    }

    struct State {
        // Held to keep the window (and its HWND that the DComp target binds) alive.
        _window: Arc<Window>,
        engine: IMFMediaEngine,
        _notify: IMFMediaEngineNotify,
        swapchain: IDXGISwapChain3,
        size: Cell<(u32, u32)>,
        _dcomp: IDCompositionDevice,
        _target: IDCompositionTarget,
        _visual: IDCompositionVisual,
    }

    #[derive(Default)]
    struct App {
        path: String,
        state: Option<State>,
    }

    impl ApplicationHandler for App {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            if self.state.is_some() {
                return;
            }

            let window = Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("agui · video_spike")
                            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT)),
                    )
                    .unwrap(),
            );

            let state = unsafe { setup(window.clone(), &self.path) };
            self.state = Some(state);
            event_loop.set_control_flow(ControlFlow::Poll);
        }

        /// Pulls and presents a frame each loop iteration, leaving the message pump free to dispatch
        /// input and the Media Engine's async events.
        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            if let Some(state) = &self.state {
                unsafe { present(state) };
            }
        }

        fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => {
                    if let Some(state) = &self.state {
                        let (w, h) = (size.width.max(1), size.height.max(1));
                        // Resize the swapchain to the window so the video fills it rather than staying
                        // its initial size; the backbuffer is only held transiently in `present`, so
                        // nothing outstanding blocks the resize.
                        unsafe {
                            state
                                .swapchain
                                .ResizeBuffers(
                                    0,
                                    w,
                                    h,
                                    DXGI_FORMAT_B8G8R8A8_UNORM,
                                    DXGI_SWAP_CHAIN_FLAG(0),
                                )
                                .expect("resize buffers");
                        }
                        state.size.set((w, h));
                        unsafe { present(state) };
                    }
                }
                WindowEvent::RedrawRequested => {
                    if let Some(state) = &self.state {
                        unsafe { present(state) };
                    }
                }
                _ => {}
            }
        }
    }

    unsafe fn setup(window: Arc<Window>, path: &str) -> State {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        MFStartup((MF_SDK_VERSION << 16) | MF_API_VERSION, MFSTARTUP_NOSOCKET).expect("mf startup");

        // A D3D11 device for Media Foundation video decode, separate from any render device — its
        // swapchain composes alongside others because DirectComposition is the shared compositor.
        let mut device: Option<ID3D11Device> = None;
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            None,
            D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_VIDEO_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_1]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            None,
        )
        .expect("d3d11 device");
        let device = device.unwrap();

        let mut token = 0u32;
        let mut manager: Option<IMFDXGIDeviceManager> = None;
        MFCreateDXGIDeviceManager(&mut token, &mut manager).expect("dxgi manager");
        let manager = manager.unwrap();
        manager.ResetDevice(&device, token).expect("reset device");

        let factory: IMFMediaEngineClassFactory =
            CoCreateInstance(&CLSID_MFMediaEngineClassFactory, None, CLSCTX_INPROC_SERVER)
                .expect("media engine factory");

        let notify: IMFMediaEngineNotify = Notify.into();

        let mut attributes: Option<IMFAttributes> = None;
        MFCreateAttributes(&mut attributes, 3).expect("attributes");
        let attributes = attributes.unwrap();
        attributes
            .SetUnknown(&MF_MEDIA_ENGINE_DXGI_MANAGER, &manager)
            .expect("dxgi manager attr");
        attributes
            .SetUnknown(&MF_MEDIA_ENGINE_CALLBACK, &notify)
            .expect("callback attr");
        attributes
            .SetUINT32(
                &MF_MEDIA_ENGINE_VIDEO_OUTPUT_FORMAT,
                DXGI_FORMAT_B8G8R8A8_UNORM.0 as u32,
            )
            .expect("output format attr");

        let engine = factory
            .CreateInstance(0, &attributes)
            .expect("media engine");
        // Autoplay + loop before the source, so it starts on its own once the media has loaded rather
        // than latching on the first frame from a too-early `Play`.
        engine.SetAutoPlay(true).expect("autoplay");
        engine.SetLoop(true).expect("set loop");
        engine.SetSource(&BSTR::from(path)).expect("set source");

        // A composition swapchain on the D3D11 device, sized to the window; Media Foundation transfers
        // each decoded frame into it.
        let dxgi: IDXGIFactory2 =
            CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)).expect("dxgi factory");
        let inner = window.inner_size();
        let (w, h) = (inner.width.max(1), inner.height.max(1));
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: w,
            Height: h,
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
        let swapchain: IDXGISwapChain3 = dxgi
            .CreateSwapChainForComposition(&device, &desc, None)
            .expect("composition swapchain")
            .cast()
            .expect("swapchain3");

        let dcomp: IDCompositionDevice =
            DCompositionCreateDevice(None::<&IDXGIDevice>).expect("dcomp device");
        let target = dcomp
            .CreateTargetForHwnd(hwnd_of(&window), true)
            .expect("dcomp target");
        let visual = dcomp.CreateVisual().expect("visual");
        visual.SetContent(&swapchain).expect("set content");
        target.SetRoot(&visual).expect("set root");
        dcomp.Commit().expect("commit");

        State {
            _window: window,
            engine,
            _notify: notify,
            swapchain,
            size: Cell::new((w, h)),
            _dcomp: dcomp,
            _target: target,
            _visual: visual,
        }
    }

    /// Pulls the current decoded frame into the swapchain and presents it.
    unsafe fn present(state: &State) {
        let _ = state.engine.OnVideoStreamTick();

        let (w, h) = state.size.get();
        let backbuffer: ID3D11Texture2D = state.swapchain.GetBuffer(0).expect("backbuffer");
        let dst = RECT {
            left: 0,
            top: 0,
            right: w as i32,
            bottom: h as i32,
        };

        // The transfer fails until a frame is decoded and queued (`OnVideoStreamTick` reports no new
        // frame as `S_FALSE`, which the wrapper cannot distinguish from `S_OK`), so present only when a
        // frame actually transferred.
        if state
            .engine
            .TransferVideoFrame(&backbuffer, None, &dst, None)
            .is_ok()
        {
            let _ = state.swapchain.Present(1, DXGI_PRESENT(0));
        }
    }

    fn hwnd_of(window: &Window) -> HWND {
        match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(handle) => HWND(handle.hwnd.get() as *mut c_void),
            other => panic!("expected a Win32 window handle, got {other:?}"),
        }
    }

    pub fn run(path: String) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut App { path, state: None }).unwrap();
    }
}
