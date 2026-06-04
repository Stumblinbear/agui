use std::{cell::Cell, num::NonZeroUsize, rc::Rc, sync::Arc};

use agui_core::{
    constraints::Constraints,
    hit_test::HitTestBehavior,
    offset::Offset,
    paint::peniko::Color,
    pointer::{PointerDispatcher, PointerEvent, PointerEventKind, PointerHandler, PointerId},
    render_object::RenderOwner,
    test_harness::TestHarness,
    widget::Widget,
};
use agui_primitives::{
    colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox, listener::Listener,
    opacity::Opacity,
};
use agui_vello::append_scene;
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
    util::{RenderContext, RenderSurface},
    wgpu,
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const CONTINUOUS_REDRAW: bool = false;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hello_world=info,agui_core=debug".into()),
        )
        .init();

    // Pointer handlers that only log, to exercise hit testing and dispatch.
    let on_down: PointerHandler =
        Rc::new(|event: &PointerEvent| tracing::info!(position = ?event.position, "pointer down"));
    let on_move: PointerHandler =
        Rc::new(|event: &PointerEvent| tracing::info!(position = ?event.position, "pointer move"));
    let on_up: PointerHandler =
        Rc::new(|event: &PointerEvent| tracing::info!(position = ?event.position, "pointer up"));

    // An orange box filling the left half of the window, listening for pointer events.
    let widget = FractionallySizedBox::new()
        .width_factor(0.5)
        .height_factor(1.0)
        .child(
            Listener::builder()
                .on_pointer_down(on_down)
                .on_pointer_move(on_move)
                .on_pointer_up(on_up)
                .behavior(HitTestBehavior::Opaque)
                .child(Opacity::new(0.5).child(ColoredBox::new(Color::rgb8(255, 138, 0)))),
        );
    let harness = TestHarness::mount(&widget);
    let child = widget.create_render_object(&harness.root.element);

    let mut owner = RenderOwner::new();

    // A dirty boundary sets this flag through the owner's hook; the driver turns it into a redraw
    // request once the event batch settles.
    let needs_redraw = Rc::new(Cell::new(false));
    let flag = Rc::clone(&needs_redraw);
    owner.on_needs_visual_update(move || flag.set(true));

    // The owner mounts the body as its root view; the driver then drives rendering through the owner.
    tracing::info!("driving mount pass");
    owner.mount_view(Box::new(child));
    tracing::info!("view mounted; starting event loop");

    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut App::new(owner, needs_redraw))
        .unwrap();
}

struct ActiveWindow {
    surface: RenderSurface<'static>,
    window: Arc<Window>,
}

struct App {
    context: RenderContext,
    /// One renderer per device; indexed by `RenderSurface::dev_id`.
    renderers: Vec<Option<Renderer>>,
    active: Option<ActiveWindow>,
    /// This window's render owner: drives layout, paint, and compositing of the view's subtree.
    owner: RenderOwner,
    /// Routes pointer events to the handlers under them, per pointer.
    dispatcher: PointerDispatcher,
    /// The most recent cursor position; `MouseInput` events carry no position of their own.
    cursor: Offset,
    /// Set by the owner's visual-update hook; drained into a redraw request in `about_to_wait`.
    needs_redraw: Rc<Cell<bool>>,
    vello_scene: vello::Scene,
}

impl App {
    fn new(owner: RenderOwner, needs_redraw: Rc<Cell<bool>>) -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
            active: None,
            owner,
            dispatcher: PointerDispatcher::new(),
            cursor: Offset::ZERO,
            needs_redraw,
            vello_scene: vello::Scene::new(),
        }
    }
}

impl ApplicationHandler for App {
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

        window.request_redraw();
        self.active = Some(ActiveWindow { surface, window });
        tracing::info!(width = size.width, height = size.height, "window created");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(active) = self.active.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                #[allow(clippy::cast_possible_truncation)]
                {
                    self.cursor = Offset::new(position.x as f32, position.y as f32);
                }

                let event = PointerEvent {
                    pointer: PointerId(0),
                    position: self.cursor,
                    kind: PointerEventKind::Move,
                };
                let owner = &self.owner;
                self.dispatcher
                    .handle(&event, |position| owner.hit_test(position));
            }

            WindowEvent::MouseInput { state, .. } => {
                let kind = match state {
                    ElementState::Pressed => PointerEventKind::Down,
                    ElementState::Released => PointerEventKind::Up,
                };

                let event = PointerEvent {
                    pointer: PointerId(0),
                    position: self.cursor,
                    kind,
                };
                let owner = &self.owner;
                self.dispatcher
                    .handle(&event, |position| owner.hit_test(position));
            }

            WindowEvent::Resized(size) => {
                tracing::info!(
                    width = size.width,
                    height = size.height,
                    "resized; marking view"
                );
                self.context
                    .resize_surface(&mut active.surface, size.width, size.height);
                // The layout changed, so the view must repaint at the new size. Marking it schedules
                // a frame through the owner's hook; no explicit redraw request here.
                self.owner.mark_needs_paint();
            }

            WindowEvent::RedrawRequested => {
                let width = active.surface.config.width;
                let height = active.surface.config.height;

                let _frame = tracing::info_span!("frame", width, height).entered();

                self.owner
                    .layout(Constraints::new(0.0, width as f32, 0.0, height as f32));

                self.owner.flush_paint();
                let scene = self.owner.composite();

                self.vello_scene.reset();
                append_scene(&scene, &mut self.vello_scene);

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

                if CONTINUOUS_REDRAW {
                    active.window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // A boundary went dirty since the last settle; ask the window to redraw.
        if self.needs_redraw.replace(false)
            && let Some(active) = &self.active
        {
            active.window.request_redraw();
        }
    }
}
