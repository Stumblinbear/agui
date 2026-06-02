//! Single window driven through the real widget pipeline: a widget tree is laid out to the window
//! size and painted into a [`Scene`] each frame, which Vello then renders.

use std::{num::NonZeroUsize, sync::Arc};

use agui_core::{
    constraints::Constraints,
    paint::{Canvas, Scene, peniko::Color},
    render_object::box_layout::RenderBox,
    test_harness::TestHarness,
    widget::Widget,
};
use agui_primitives::{colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox};
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
    util::{RenderContext, RenderSurface},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

fn main() {
    // An orange box filling the left half of the window, via a fractionally sized box.
    let widget = FractionallySizedBox::new()
        .width_factor(0.5_f32)
        .height_factor(1.0_f32)
        .child(ColoredBox::new(Color::rgb8(255, 138, 0)));
    let harness = TestHarness::mount(&widget);
    let render = widget.create_render_object(&harness.root.element);

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::new(render)).unwrap();
}

struct ActiveWindow {
    surface: RenderSurface<'static>,
    window: Arc<Window>,
}

struct App<R> {
    context: RenderContext,
    /// One renderer per device; indexed by `RenderSurface::dev_id`.
    renderers: Vec<Option<Renderer>>,
    active: Option<ActiveWindow>,
    /// The root render object, laid out and painted each frame.
    render: R,
    /// Reused across frames so the paint pass doesn't reallocate.
    scene: Scene,
    vello_scene: vello::Scene,
}

impl<R> App<R> {
    fn new(render: R) -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
            active: None,
            render,
            scene: Scene::new(),
            vello_scene: vello::Scene::new(),
        }
    }
}

impl<R: RenderBox> ApplicationHandler for App<R> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.active.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("agui · hello_world")
            .with_inner_size(LogicalSize::new(800, 600));
        let window = Arc::new(event_loop.create_window(attributes).unwrap());

        let size = window.inner_size();
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
                    surface_format: Some(surface.format),
                    use_cpu: false,
                    antialiasing_support: AaSupport::area_only(),
                    num_init_threads: NonZeroUsize::new(1),
                },
            )
            .unwrap()
        });

        self.active = Some(ActiveWindow { surface, window });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(active) = self.active.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                self.context
                    .resize_surface(&mut active.surface, size.width, size.height);
                active.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let width = active.surface.config.width;
                let height = active.surface.config.height;

                self.render
                    .layout(Constraints::new(0.0, width as f32, 0.0, height as f32));

                Canvas::record_into(&mut self.scene, |canvas| {
                    self.render.paint(canvas);
                });

                self.vello_scene.reset();
                agui_vello::append_scene(&self.scene, &mut self.vello_scene);

                let device = &self.context.devices[active.surface.dev_id];
                let texture = active.surface.surface.get_current_texture().unwrap();
                self.renderers[active.surface.dev_id]
                    .as_mut()
                    .unwrap()
                    .render_to_surface(
                        &device.device,
                        &device.queue,
                        &self.vello_scene,
                        &texture,
                        &RenderParams {
                            base_color: Color::rgb8(30, 30, 30),
                            width,
                            height,
                            antialiasing_method: AaConfig::Area,
                        },
                    )
                    .unwrap();
                texture.present();
            }
            _ => {}
        }
    }
}
