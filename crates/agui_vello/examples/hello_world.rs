use std::{
    cell::RefCell,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use agui_core::{
    input::pointer::{PointerDispatcher, PointerHandler},
    paint::{
        compositing::{ContainerLayer, LayerHandle},
        peniko::kurbo::Affine,
        scene::Scene,
    },
    pipeline::{PipelineOwner, build::BuildOwner, layout::BoundaryContent},
    prelude::{element::*, render_object::*},
    scheduling::{LocalReactor, Vsync},
};
use agui_primitives::{
    animated_transform::AnimatedTransform, colored_box::ColoredBox,
    fractionally_sized_box::FractionallySizedBox, layout_builder::LayoutBuilder,
    listener::Listener, opacity::Opacity,
};
use agui_vello::append_scene;
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
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hello_world=info,agui_core=debug".into()),
        )
        .init();

    // An orange box filling the left half of the window, listening for pointer events.
    let ui = LayoutBuilder::new(|constraints| {
        if constraints.max_width().get() < 400.0 {
            return ColoredBox::new(Color::rgb8(255, 138, 0)).into_boxed_render_box();
        }

        // Pointer handlers that only log, to exercise hit testing and dispatch.
        let on_down: PointerHandler = Rc::new(
            |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer down"),
        );
        let on_move: PointerHandler = Rc::new(
            |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer move"),
        );
        let on_up: PointerHandler = Rc::new(
            |event: &PointerEvent| tracing::info!(position = ?event.position, "pointer up"),
        );

        FractionallySizedBox::new()
            .width_factor(0.5)
            .height_factor(1.0)
            .child(
                Listener::builder()
                    .on_pointer_down(on_down)
                    .on_pointer_move(on_move)
                    .on_pointer_up(on_up)
                    .behavior(HitTestBehavior::Opaque)
                    .child(Opacity::new(0.5).child(ColoredBox::new(Color::rgb8(255, 138, 0)))),
            )
            .into_boxed_render_box()
    });

    let vsync = Vsync::new();
    let content = AnimatedTransform::new(|now| {
        let pivot = Affine::translate((400.0, 300.0));
        pivot * Affine::rotate(now.as_secs_f64()) * pivot.inverse()
    })
    .vsync(vsync.clone())
    .child(ui);

    tracing::info!("mounting window");

    let event_loop = EventLoop::<WakeUp>::with_user_event().build().unwrap();

    // The reactor drives spawned tasks; when one becomes ready it posts a WakeUp through the proxy so
    // this loop, otherwise asleep, comes back to advance it.
    let proxy = event_loop.create_proxy();
    let reactor = LocalReactor::new(move || {
        let _ = proxy.send_event(WakeUp);
    });

    // The driver owns the pipeline for the subtree and hands its presentation layer back out here. The
    // OS surface would normally take that layer; this example presents it by compositing each frame.
    let driver = WindowDriver::new(
        content,
        reactor,
        vsync,
        |_layer: LayerHandle<ContainerLayer>| {},
    );
    let view: Box<dyn View> = Box::new(driver);

    tracing::info!("window mounted; starting event loop");
    event_loop.run_app(&mut App::new(view)).unwrap();
}

/// Posted by the reactor through the event-loop proxy to wake the loop when a spawned task is ready.
struct WakeUp;

/// The loose constraints a window of `width` by `height` lays its subtree out under.
#[allow(clippy::cast_precision_loss)]
fn viewport(width: u32, height: u32) -> BoxConstraints {
    BoxConstraints::tight(Size::new(width, height))
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
    /// The window's view: it sizes, lays out, paints, composites, and hit-tests the subtree.
    view: Box<dyn View>,
    /// Routes pointer events to the handlers under them, per pointer.
    dispatcher: PointerDispatcher,
    /// The most recent cursor position; `MouseInput` events carry no position of their own.
    cursor: Offset,
    /// Whether a frame has been laid out and painted yet. Hit testing reads layout, so it waits for
    /// the first frame.
    painted: bool,
    /// The instant the app started, the origin for the frame time handed to each [`View::frame`].
    start: Instant,
    vello_scene: vello::Scene,
}

impl App {
    fn new(view: Box<dyn View>) -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
            active: None,
            view,
            dispatcher: PointerDispatcher::new(),
            cursor: Offset::ZERO,
            painted: false,
            start: Instant::now(),
            vello_scene: vello::Scene::new(),
        }
    }

    /// Lays out, paints, and presents a frame, revealing the window after its first one.
    fn draw(&mut self) {
        let Some(active) = self.active.as_mut() else {
            return;
        };

        let width = active.surface.config.width;
        let height = active.surface.config.height;

        let _frame = tracing::info_span!("frame", width, height).entered();

        let scene = self.view.frame(self.start.elapsed());

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

        // The first frame is on screen; reveal the window and start accepting pointer events.
        if !self.painted {
            active.window.set_visible(true);

            self.painted = true;
        }

        // While frame callbacks are registered, request the next frame so animations keep advancing.
        // AutoVsync paces these to the display.
        if self.view.is_animating() {
            active.window.request_redraw();
        }
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

        self.view.poll_tasks();

        if self.view.needs_frame()
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
            .with_title("agui · hello_world")
            .with_inner_size(LogicalSize::new(800, 600))
            .with_visible(false);

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

        // Lay out, paint, and reveal the first frame inline; a hidden window receives no redraw request.
        self.view.resize(viewport(size.width, size.height));
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
                #[allow(clippy::cast_possible_truncation)]
                {
                    self.cursor = Offset::new(position.x as f32, position.y as f32);
                }

                if self.painted {
                    let event = PointerEvent {
                        pointer: PointerId(0),
                        position: self.cursor,
                        kind: PointerEventKind::Move,
                    };
                    let view = &self.view;
                    self.dispatcher
                        .handle(&event, |position| view.hit_test(position));
                }
            }

            WindowEvent::MouseInput { state, .. } => {
                if !self.painted {
                    return;
                }

                let kind = match state {
                    ElementState::Pressed => PointerEventKind::Down,
                    ElementState::Released => PointerEventKind::Up,
                };

                let event = PointerEvent {
                    pointer: PointerId(0),
                    position: self.cursor,
                    kind,
                };
                let view = &self.view;
                self.dispatcher
                    .handle(&event, |position| view.hit_test(position));
            }

            WindowEvent::Resized(size) => {
                tracing::info!(width = size.width, height = size.height, "resized");
                if let Some(active) = self.active.as_mut() {
                    self.context
                        .resize_surface(&mut active.surface, size.width, size.height);
                }

                // The viewport changed, so re-lay and repaint the subtree at the new size.
                self.view.resize(viewport(size.width, size.height));

                self.draw();
            }

            WindowEvent::RedrawRequested => self.draw(),
            _ => {}
        }
    }
}

/// Pairs a [`BuildOwner`], which holds the element tree the widget describes, with a [`PipelineOwner`],
/// which lays out and paints the matching render tree, to drive one window from the event loop.
///
/// This is the shape a real window binding takes: the build owner rebuilds the element tree when
/// something asks it to, the pipeline owner brings layout and paint up to date, and each frame keeps the
/// two in step before compositing for presentation. `on_layer_created` receives the layer the window
/// presents.
struct WindowDriver<V: Widget>
where
    V::Render: RenderBox + 'static,
{
    reactor: LocalReactor,
    vsync: Vsync,
    build: BuildOwner<V>,
    content: BoundaryContent,
    owner: PipelineOwner,
}

impl<V: Widget> WindowDriver<V>
where
    V::Render: RenderBox + 'static,
{
    fn new(
        widget: V,
        reactor: LocalReactor,
        vsync: Vsync,
        on_layer_created: impl FnOnce(LayerHandle<ContainerLayer>),
    ) -> Self {
        let mut build = BuildOwner::mount(widget, &mut reactor.scheduler());

        // Seed the render tree from the element tree and register its root as the pipeline's outermost
        // boundary.
        let content: BoundaryContent = Rc::new(RefCell::new(build.create_render_object()));
        let layer = LayerHandle::new(ContainerLayer::new());

        let owner = PipelineOwner::new(Rc::clone(&content), layer.clone());

        on_layer_created(layer);

        Self {
            reactor,
            vsync,
            build,
            content,
            owner,
        }
    }

    fn sync_render(&mut self) {
        let mut content = self.content.borrow_mut();
        let render = content
            .as_any_mut()
            .downcast_mut::<V::Render>()
            .expect("the root render object keeps its type");

        self.build.update_render_object(render);
    }
}

/// Drives one window's pipeline from the event loop.
trait View {
    fn resize(&mut self, constraints: BoxConstraints);
    /// Advances spawned tasks as far as they will go and applies the messages they post.
    fn poll_tasks(&mut self);
    /// Whether a frame is owed: the tree was dirtied or an animation is running.
    fn needs_frame(&self) -> bool;
    fn frame(&mut self, now: Duration) -> Scene;
    /// Whether a frame callback is registered, so the loop should keep requesting frames.
    fn is_animating(&self) -> bool;
    fn hit_test(&self, position: Offset) -> HitTestResult;
}

impl<V: Widget> View for WindowDriver<V>
where
    V::Render: RenderBox + 'static,
{
    fn resize(&mut self, constraints: BoxConstraints) {
        self.owner.resize(constraints);
    }

    fn poll_tasks(&mut self) {
        // Keep polling while tasks make progress or post messages, so a chain of wakeups settles in
        // one pass rather than one per frame. Dispatching a message may ready a task, so both are
        // drained together.
        loop {
            let ran = self.reactor.poll();

            let messages = self.reactor.messages().collect::<Vec<_>>();
            let delivered = !messages.is_empty();
            for (path, message) in messages {
                self.build.dispatch_message(path, message);
            }

            if !ran && !delivered {
                break;
            }
        }
    }

    fn needs_frame(&self) -> bool {
        self.build.is_dirty() || !self.vsync.is_idle()
    }

    fn frame(&mut self, now: Duration) -> Scene {
        // Tasks have already been drained, so apply any rebuild they queued, run frame callbacks for
        // this frame's time, then lay out and paint what changed.
        if self.build.flush(&mut self.reactor.scheduler()) {
            self.sync_render();
        }

        self.vsync.tick(now);

        self.owner.flush_layout();
        self.owner.flush_paint();
        self.owner.composite()
    }

    fn is_animating(&self) -> bool {
        !self.vsync.is_idle()
    }

    fn hit_test(&self, position: Offset) -> HitTestResult {
        self.owner.hit_test(position)
    }
}
