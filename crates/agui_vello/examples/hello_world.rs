use std::{cell::RefCell, num::NonZeroUsize, rc::Rc, sync::Arc};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::{Element, ElementNode},
    hit_test::{HitTestBehavior, HitTestResult},
    offset::Offset,
    paint::{ContainerLayer, LayerHandle, Scene, peniko::Color},
    pointer::{PointerDispatcher, PointerEvent, PointerEventKind, PointerHandler, PointerId},
    render_object::{BoundaryContent, PipelineOwner, box_layout::RenderBox},
    routing_id::RoutingId,
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
    let content = FractionallySizedBox::new()
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

    // The window owns the pipeline for its subtree and hands its presentation layer back out here. The
    // OS surface would normally take that layer; this example presents it by compositing each frame.
    let widget = WindowRoot {
        on_layer_created: |_layer: LayerHandle<ContainerLayer>| {},
        child: content,
    };

    tracing::info!("mounting window");
    let harness = TestHarness::mount(&widget);
    let view: Box<dyn View> = Box::new(harness.root.element);
    tracing::info!("window mounted; starting event loop");

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::new(view)).unwrap();
}

/// The loose constraints a window of `width` by `height` lays its subtree out under.
#[allow(clippy::cast_precision_loss)]
fn viewport(width: u32, height: u32) -> Constraints {
    Constraints::new(0.0, width as f32, 0.0, height as f32)
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

/// A widget that owns a window's render pipeline and presents its child filling the window.
///
/// Its child becomes the root of a render tree detached from the surrounding element tree, driven by a
/// [`PipelineOwner`] the element holds. `on_layer_created` receives the layer the window presents.
struct WindowRoot<OnLayerCreated, Child> {
    on_layer_created: OnLayerCreated,
    child: Child,
}

impl<OnLayerCreated, Child> Widget for WindowRoot<OnLayerCreated, Child>
where
    OnLayerCreated: Fn(LayerHandle<ContainerLayer>),
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = WindowRootElement<Child::Element>;
    type Render = ();

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        let mut child = ElementNode::new(self.child.create_element(ctx));

        // Erase the child render object into the shared boundary cell the owner registers as the root.
        let content: BoundaryContent =
            Rc::new(RefCell::new(child.create_render_object(&self.child)));
        let layer = LayerHandle::new(ContainerLayer::new());

        let owner = PipelineOwner::new(Rc::clone(&content), layer.clone());

        (self.on_layer_created)(layer);

        WindowRootElement {
            child,
            content,
            owner,
        }
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child.update(&self.child, &old.child, ctx);

        // The render tree is detached under the owner, so reconcile it here, recovering the child's
        // concrete type from the shared boundary cell.
        let mut content = element.content.borrow_mut();
        let child = content
            .as_any_mut()
            .downcast_mut::<Child::Render>()
            .expect("the window child keeps its type across reconcile");
        element.child.update_render_object(&self.child, child);
    }

    fn create_render_object(&self, _: &Self::Element) -> Self::Render {}

    fn update_render_object(&self, _: &Self::Element, (): &mut Self::Render) {}

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.child.dispatch(&self.child, path, action);
    }
}

struct WindowRootElement<ChildElement> {
    child: ElementNode<ChildElement>,
    content: BoundaryContent,
    owner: PipelineOwner,
}

impl<ChildElement: Element> Element for WindowRootElement<ChildElement> {}

/// Drives one window's pipeline from the event loop.
trait View {
    fn resize(&mut self, constraints: Constraints);
    fn frame(&mut self) -> Scene;
    fn hit_test(&self, position: Offset) -> HitTestResult;
}

impl<ChildElement: Element> View for WindowRootElement<ChildElement> {
    fn resize(&mut self, constraints: Constraints) {
        self.owner.resize(constraints);
    }

    fn frame(&mut self) -> Scene {
        self.owner.flush_layout();
        self.owner.flush_paint();
        self.owner.composite()
    }

    fn hit_test(&self, position: Offset) -> HitTestResult {
        self.owner.hit_test(position)
    }
}
