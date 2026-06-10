use std::{
    num::NonZeroUsize,
    rc::Rc,
    sync::{Arc, mpsc},
    task::{Context, Wake, Waker},
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
    provide::Provide,
    scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler, Vsync},
};
use agui_primitives::{
    animated_transform::AnimatedTransform, colored_box::ColoredBox,
    fractionally_sized_box::FractionallySizedBox, layout_builder::LayoutBuilder,
    listener::Listener, opacity::Opacity, text::Text,
};
use agui_vello::append_scene_with_transform;
use async_executor::LocalExecutor;
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
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hello_world=info,agui_core=debug".into()),
        )
        .init();

    let vsync = Vsync::new();

    let ui = {
        let vsync = vsync.clone();

        Provide::new(Rc::new(Fonts::new())).child(LayoutBuilder::new(move |constraints| {
            if constraints.max_width().get() < 900.0 {
                return ColoredBox::new(Color::from_rgb8(255, 138, 0))
                    .child(Text::new("Hello, world!").family("Arial"))
                    .into_boxed_render_box();
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
                        .child(
                            AnimatedTransform::new(|now| Affine::rotate(now.as_secs_f64()))
                                .vsync(vsync.clone())
                                .alignment(Alignment::CENTER)
                                .child(
                                    Opacity::new(0.5).child(
                                        ColoredBox::new(Color::from_rgb8(255, 138, 0))
                                            .child(Text::new("Hello, world!").family("Arial")),
                                    ),
                                ),
                        ),
                )
                .into_boxed_render_box()
        }))
    };

    tracing::info!("mounting window");

    let event_loop = EventLoop::<WakeUp>::with_user_event().build().unwrap();

    // A single-threaded executor drives spawned tasks. Its waker posts a WakeUp through the proxy so
    // this loop, otherwise asleep, comes back to advance a task that became ready off-thread.
    let proxy = event_loop.create_proxy();
    let waker = Waker::from(Arc::new(ProxyWaker(proxy)));
    let executor = Rc::new(LocalExecutor::new());
    let (event_tx, events_rx) = mpsc::channel::<TaskEventMessage>();

    // The driver owns the pipeline for the subtree and hands its presentation layer back out here. The
    // OS surface would normally take that layer; this example presents it by compositing each frame.
    let driver = WindowDriver::new(
        ui,
        executor,
        event_tx,
        events_rx,
        waker,
        vsync,
        |_layer: LayerHandle<ContainerLayer>| {},
    );
    let view: Box<dyn View> = Box::new(driver);

    tracing::info!("window mounted; starting event loop");
    event_loop.run_app(&mut App::new(view)).unwrap();
}

/// Posted through the event-loop proxy to wake the loop when a spawned task becomes ready.
struct WakeUp;

/// Bridges the executor's wakeups to the event loop: a task's waker calls this, and it posts a
/// `WakeUp` through the proxy so an otherwise-sleeping loop comes back to tick the ready task.
struct ProxyWaker(EventLoopProxy<WakeUp>);

impl Wake for ProxyWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        let _ = self.0.send_event(WakeUp);
    }
}

/// A [`TaskScheduler`] that spawns onto a single-threaded [`LocalExecutor`] and posts task messages back
/// through `event_tx`. Cloning shares the executor, so it is free of borrows and can be captured to
/// spawn later, including during layout.
struct ExecutorScheduler {
    executor: Rc<LocalExecutor<'static>>,
    event_tx: EventSender,
}

impl TaskScheduler for ExecutorScheduler {
    fn event_tx(&self) -> EventSender {
        self.event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        // Holding the returned handle keeps the task alive; dropping it drops the `Task`, which cancels.
        let task = self.executor.spawn(func);

        Ok(TaskHandle::new(Box::new(move || drop(task))))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(ExecutorScheduler {
            executor: Rc::clone(&self.executor),
            event_tx: self.event_tx.clone(),
        })
    }
}

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
        let scale_factor = active.scale_factor;

        let _frame = tracing::info_span!("frame", width, height).entered();

        let scene = self.view.frame(self.start.elapsed());

        self.vello_scene.reset();
        append_scene_with_transform(&scene, &mut self.vello_scene, Affine::scale(scale_factor));

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
        self.view
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
                let scale_factor = if let Some(active) = self.active.as_mut() {
                    self.context
                        .resize_surface(&mut active.surface, size.width, size.height);
                    active.scale_factor
                } else {
                    return;
                };

                // The viewport changed, so re-lay and repaint the subtree at the new size.
                self.view
                    .resize(viewport(size.width, size.height, scale_factor));

                self.draw();
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                tracing::info!(scale_factor, "scale factor changed");

                // A resize follows on most platforms, but re-lay out here so the subtree tracks the new
                // density even when the physical size is unchanged.
                let size = if let Some(active) = self.active.as_mut() {
                    active.scale_factor = scale_factor;
                    (active.surface.config.width, active.surface.config.height)
                } else {
                    return;
                };

                self.view.resize(viewport(size.0, size.1, scale_factor));
                self.draw();
            }

            WindowEvent::RedrawRequested => {
                self.draw();

                // Sustain the frame loop from here rather than only from `about_to_wait`: the Win32 modal
                // resize loop pumps `WM_PAINT` but never lets `about_to_wait` run, so re-requesting on each
                // draw is what keeps an animation turning while the edge is held.
                if self.view.needs_frame()
                    && let Some(active) = self.active.as_ref()
                {
                    active.window.request_redraw();
                }
            }
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
struct WindowDriver {
    executor: Rc<LocalExecutor<'static>>,
    event_tx: EventSender,
    events_rx: mpsc::Receiver<TaskEventMessage>,
    waker: Waker,
    vsync: Vsync,
    build: BuildOwner,
    owner: PipelineOwner,
}

impl WindowDriver {
    fn new<V>(
        widget: V,
        executor: Rc<LocalExecutor<'static>>,
        event_tx: EventSender,
        events_rx: mpsc::Receiver<TaskEventMessage>,
        waker: Waker,
        vsync: Vsync,
        on_layer_created: impl FnOnce(LayerHandle<ContainerLayer>),
    ) -> Self
    where
        V: Widget + 'static,
        V::Render: RenderBox,
    {
        // Build the element tree and its root render object together, registering the root as the
        // pipeline's outermost boundary.
        let mut scheduler = ExecutorScheduler {
            executor: Rc::clone(&executor),
            event_tx: event_tx.clone(),
        };
        let (build, render) = BuildOwner::mount(widget, &mut scheduler);

        let content: BoundaryContent = render;
        let layer = LayerHandle::new(ContainerLayer::new());

        let owner = PipelineOwner::new(content, layer.clone());

        on_layer_created(layer);

        Self {
            executor,
            event_tx,
            events_rx,
            waker,
            vsync,
            build,
            owner,
        }
    }

    fn scheduler(&self) -> ExecutorScheduler {
        ExecutorScheduler {
            executor: Rc::clone(&self.executor),
            event_tx: self.event_tx.clone(),
        }
    }
}

/// Drives one window's pipeline from the event loop.
trait View {
    fn resize(&mut self, constraints: BoxConstraints);
    /// Advances spawned tasks as far as they will go and applies the messages they post.
    fn poll_tasks(&mut self);
    /// Whether a frame is owed: the tree was dirtied or an animation is still ticking.
    fn needs_frame(&self) -> bool;
    fn frame(&mut self, now: Duration) -> Scene;
    fn hit_test(&self, position: Offset) -> HitTestResult;
}

impl View for WindowDriver {
    fn resize(&mut self, constraints: BoxConstraints) {
        self.owner.resize(constraints);
    }

    fn poll_tasks(&mut self) {
        // Keep polling while tasks make progress or post messages, so a chain of wakeups settles in
        // one pass rather than one per frame. Dispatching a message may ready a task, so both are
        // drained together.
        let mut cx = Context::from_waker(&self.waker);

        loop {
            // `tick` readies one task per poll; drain every ready one. When it goes pending the proxy
            // waker is armed, so a later off-thread wakeup re-posts `WakeUp` and brings the loop back.
            let mut ran = false;
            while std::pin::pin!(self.executor.tick())
                .poll(&mut cx)
                .is_ready()
            {
                ran = true;
            }

            let messages = self.events_rx.try_iter().collect::<Vec<_>>();
            let delivered = !messages.is_empty();
            for (path, message) in messages {
                self.build.dispatch_message(&path, message);
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
        // Tasks have already been drained, so apply any rebuild they queued, advance frame callbacks for
        // this frame's time, then lay out and paint what changed.
        let mut scheduler = self.scheduler();
        self.build.flush(&mut scheduler);

        self.vsync.tick(now);

        self.owner.flush_layout();
        self.owner.flush_paint();
        self.owner.composite()
    }

    fn hit_test(&self, position: Offset) -> HitTestResult {
        self.owner.hit_test(position)
    }
}
