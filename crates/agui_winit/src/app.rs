use std::{num::NonZeroUsize, sync::Arc, time::Instant};

use agui_core::{
    input::pointer::{PointerDispatcher, PointerEvent, PointerEventKind, PointerId},
    paint::peniko::kurbo::Affine,
    prelude::{element::*, render_object::*},
};
use agui_vello::append_scene_with_transform;
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
    peniko::Color,
    util::{RenderContext, RenderSurface},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

use crate::{WindowOptions, driver::WakeUp, driver::WindowDriver};

/// The constraints a window's subtree lays out under, in logical pixels, converting the physical
/// `width` by `height` of its surface by `scale_factor`.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn viewport(width: u32, height: u32, scale_factor: f64) -> BoxConstraints {
    let scale = scale_factor as f32;
    BoxConstraints::tight(Size::new(width as f32 / scale, height as f32 / scale))
}

struct ActiveWindow {
    surface: RenderSurface<'static>,
    window: Arc<Window>,
    scale_factor: f64,
}

/// The winit application that hosts one window: it creates the surface, drives the pipeline each
/// frame, rasterizes the composed frame with vello, and routes pointer input to the subtree.
pub(crate) struct App {
    options: WindowOptions,
    driver: WindowDriver,

    context: RenderContext,
    /// One renderer per device; indexed by [`RenderSurface::dev_id`].
    renderers: Vec<Option<Renderer>>,
    active: Option<ActiveWindow>,
    vello_scene: vello::Scene,

    /// Routes pointer events to the handlers under them, per pointer.
    dispatcher: PointerDispatcher,
    /// The most recent cursor position; `MouseInput` events carry no position of their own.
    cursor: Offset,
    /// Whether a frame has been laid out and painted yet. Hit testing reads layout, so it waits for
    /// the first frame.
    painted: bool,
    /// The instant the app started, the origin for the frame time handed to each frame.
    start: Instant,
}

impl App {
    pub(crate) fn new(options: WindowOptions, driver: WindowDriver) -> Self {
        Self {
            options,
            driver,
            context: RenderContext::new(),
            renderers: Vec::new(),
            active: None,
            vello_scene: vello::Scene::new(),
            dispatcher: PointerDispatcher::new(),
            cursor: Offset::ZERO,
            painted: false,
            start: Instant::now(),
        }
    }

    /// Lays out, paints, and presents a frame, revealing the window after its first one.
    fn draw(&mut self) {
        let Some(active) = self.active.as_ref() else {
            return;
        };

        let width = active.surface.config.width;
        let height = active.surface.config.height;
        let scale_factor = active.scale_factor;

        let _frame = tracing::info_span!("frame", width, height).entered();

        let frame = self.driver.frame(self.start.elapsed());

        self.vello_scene.reset();
        let base = Affine::scale(scale_factor);
        // This window has no system compositor, so rasterize the whole frame into one scene.
        append_scene_with_transform(&frame.rasterize(), &mut self.vello_scene, base);

        let device = &self.context.devices[active.surface.dev_id];
        let surface = &active.surface;

        self.renderers[surface.dev_id]
            .as_mut()
            .unwrap()
            .render_to_texture(
                &device.device,
                &device.queue,
                &self.vello_scene,
                &surface.target_view,
                &RenderParams {
                    base_color: Color::from_rgb8(30, 30, 30),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .unwrap();

        let texture = surface.surface.get_current_texture().unwrap();
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        surface.blitter.copy(
            &device.device,
            &mut encoder,
            &surface.target_view,
            &texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device.queue.submit([encoder.finish()]);
        texture.present();

        // The first frame is on screen; reveal the window and start accepting pointer events.
        if !self.painted {
            active.window.set_visible(true);
            self.painted = true;
        }
    }

    /// Routes a pointer event through the subtree, once the first frame has painted.
    fn dispatch_pointer(&mut self, kind: PointerEventKind) {
        if !self.painted {
            return;
        }

        let event = PointerEvent {
            pointer: PointerId(0),
            position: self.cursor,
            kind,
        };
        let driver = &self.driver;
        self.dispatcher
            .handle(&event, |position| driver.hit_test(position));
    }
}

impl ApplicationHandler<WakeUp> for App {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: WakeUp) {
        // A task became ready. Delivering this event has woken the loop; `about_to_wait` drains the
        // reactor and decides whether a frame is needed, so there is nothing to do here.
    }

    /// Drains tasks before the loop sleeps. Polling runs independently of painting, so a burst of
    /// task wakeups settles in one pass; a frame is requested only if that left the tree dirty or an
    /// animation running.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if self.active.is_none() {
            return;
        }

        self.driver.poll_tasks();

        if self.driver.needs_frame()
            && let Some(active) = self.active.as_ref()
        {
            active.window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.active.is_some() {
            return;
        }

        // Start hidden and reveal the window once its first frame has painted, to avoid a blank flash.
        let attributes = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(LogicalSize::new(self.options.width, self.options.height))
            .with_visible(false);

        let window = Arc::new(event_loop.create_window(attributes).unwrap());

        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let surface = pollster::block_on(self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        ))
        .unwrap();

        self.renderers
            .resize_with(self.context.devices.len(), || None);
        self.renderers[surface.dev_id].get_or_insert_with(|| {
            Renderer::new(
                &self.context.devices[surface.dev_id].device,
                RendererOptions {
                    use_cpu: false,
                    antialiasing_support: AaSupport::area_only(),
                    num_init_threads: NonZeroUsize::new(1),
                    pipeline_cache: None,
                },
            )
            .unwrap()
        });

        self.active = Some(ActiveWindow {
            surface,
            window,
            scale_factor,
        });

        // Lay out, paint, and reveal the first frame inline; a hidden window receives no redraw request.
        self.driver
            .resize(viewport(size.width, size.height, scale_factor));
        self.draw();

        tracing::info!(width = size.width, height = size.height, "window created");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.active.is_none() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.active.as_ref().map_or(1.0, |a| a.scale_factor);
                let logical = position.to_logical::<f64>(scale);
                #[allow(clippy::cast_possible_truncation)]
                {
                    self.cursor = Offset::new(logical.x as f32, logical.y as f32);
                }

                self.dispatch_pointer(PointerEventKind::Move);
            }

            WindowEvent::MouseInput { state, .. } => {
                let kind = match state {
                    ElementState::Pressed => PointerEventKind::Down,
                    ElementState::Released => PointerEventKind::Up,
                };

                self.dispatch_pointer(kind);
            }

            WindowEvent::Resized(size) => {
                tracing::info!(width = size.width, height = size.height, "resized");
                let scale_factor = if let Some(active) = self.active.as_mut() {
                    self.context
                        .resize_surface(&mut active.surface, size.width, size.height);
                    active.scale_factor
                } else {
                    return;
                };

                // The viewport changed, so re-lay and repaint the subtree at the new size.
                self.driver
                    .resize(viewport(size.width, size.height, scale_factor));
                self.draw();
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                tracing::info!(scale_factor, "scale factor changed");

                // A resize follows on most platforms, but re-lay out here so the subtree tracks the
                // new density even when the physical size is unchanged.
                let size = if let Some(active) = self.active.as_mut() {
                    active.scale_factor = scale_factor;
                    (active.surface.config.width, active.surface.config.height)
                } else {
                    return;
                };

                self.driver.resize(viewport(size.0, size.1, scale_factor));
                self.draw();
            }

            WindowEvent::RedrawRequested => {
                self.draw();

                // Sustain the frame loop from here rather than only from `about_to_wait`: the Win32
                // modal resize loop pumps `WM_PAINT` but never lets `about_to_wait` run, so
                // re-requesting on each draw is what keeps an animation turning while the edge is held.
                if self.driver.needs_frame()
                    && let Some(active) = self.active.as_ref()
                {
                    active.window.request_redraw();
                }
            }

            _ => {}
        }
    }
}
