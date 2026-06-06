use std::{cell::RefCell, num::NonZeroUsize, rc::Rc, sync::Arc};

use agui_core::{
    input::pointer::{PointerDispatcher, PointerHandler},
    paint::{
        compositing::{ContainerLayer, LayerHandle},
        scene::Scene,
    },
    pipeline::{PipelineOwner, build::BuildOwner, layout::BoundaryContent},
    prelude::{element::*, render_object::*},
    test_harness::TestTaskRunner,
};
use agui_primitives::{
    colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox,
    layout_builder::LayoutBuilder, listener::Listener, opacity::Opacity,
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
    let content = LayoutBuilder::new(|constraints| {
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

    tracing::info!("mounting window");
    // The driver owns the pipeline for the subtree and hands its presentation layer back out here. The
    // OS surface would normally take that layer; this example presents it by compositing each frame.
    let driver = WindowDriver::new(content, |_layer: LayerHandle<ContainerLayer>| {});
    let view: Box<dyn View> = Box::new(driver);
    tracing::info!("window mounted; starting event loop");

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::new(view)).unwrap();
}

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

        let scene = self.view.frame();

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
    }
}

impl ApplicationHandler for App {
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
    tasks: TestTaskRunner,
    build: BuildOwner<V>,
    content: BoundaryContent,
    owner: PipelineOwner,
}

impl<V: Widget> WindowDriver<V>
where
    V::Render: RenderBox + 'static,
{
    fn new(widget: V, on_layer_created: impl FnOnce(LayerHandle<ContainerLayer>)) -> Self {
        let mut tasks = TestTaskRunner::new();

        let mut build = BuildOwner::mount(widget, &mut tasks.scheduler());

        // Seed the render tree from the element tree and register its root as the pipeline's outermost
        // boundary.
        let content: BoundaryContent = Rc::new(RefCell::new(build.create_render_object()));
        let layer = LayerHandle::new(ContainerLayer::new());

        let owner = PipelineOwner::new(Rc::clone(&content), layer.clone());

        on_layer_created(layer);

        Self {
            tasks,
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
    fn frame(&mut self) -> Scene;
    fn hit_test(&self, position: Offset) -> HitTestResult;
}

impl<V: Widget> View for WindowDriver<V>
where
    V::Render: RenderBox + 'static,
{
    fn resize(&mut self, constraints: BoxConstraints) {
        self.owner.resize(constraints);
    }

    fn frame(&mut self) -> Scene {
        // Run spawned tasks one step, deliver the messages they posted, and rebuild whatever they
        // dirtied, bringing the render tree back in step before laying out and painting.
        self.tasks.poll();

        let messages = self.tasks.messages().collect::<Vec<_>>();
        for (path, message) in messages {
            self.build.dispatch_message(path, message);
        }

        if self.build.flush(&mut self.tasks.scheduler()) {
            self.sync_render();
        }

        self.owner.flush_layout();
        self.owner.flush_paint();
        self.owner.composite()
    }

    fn hit_test(&self, position: Offset) -> HitTestResult {
        self.owner.hit_test(position)
    }
}
