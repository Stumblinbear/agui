use std::{cell::RefCell, num::NonZeroUsize, rc::Rc, sync::Arc};

use agui_core::{
    constraints::Constraints,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::{ContainerLayer, LayerHandle, PaintCtx, peniko::Color},
    render_object::{
        BoundaryContent, MountCtx, PaintScope, RenderObject, RepaintOwner,
        box_layout::{AnyRenderBox, RenderBox},
    },
    size::Size,
    test_harness::TestHarness,
    text_baseline::TextBaseline,
    widget::Widget,
};
use agui_primitives::{colored_box::ColoredBox, fractionally_sized_box::FractionallySizedBox};
use agui_vello::append_scene;
use typed_floats::{Positive, PositiveFinite};
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

const CONTINUOUS_REDRAW: bool = false;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hello_world=info,agui_core=debug".into()),
        )
        .init();

    // An orange box filling the left half of the window.
    let widget = FractionallySizedBox::new()
        .width_factor(0.5_f32)
        .height_factor(1.0_f32)
        .child(ColoredBox::new(Color::rgb8(255, 138, 0)));
    let harness = TestHarness::mount(&widget);
    let child = widget.create_render_object(&harness.root.element);

    let mut owner = RepaintOwner::new();

    // The root boundary hands its paint scope back through this slot when it mounts.
    let scope_slot: Rc<RefCell<Option<PaintScope>>> = Rc::new(RefCell::new(None));
    let slot = Rc::clone(&scope_slot);
    let mut render = RootBoundary::new(child, move |scope| {
        *slot.borrow_mut() = Some(scope);
    });

    tracing::info!("driving mount pass");
    render.mount(&mut MountCtx::new(&mut owner));
    let root_scope = scope_slot
        .borrow()
        .clone()
        .expect("the root boundary mounted");
    tracing::info!("root boundary mounted; starting event loop");

    let event_loop = EventLoop::new().unwrap();
    event_loop
        .run_app(&mut App::new(owner, render, root_scope))
        .unwrap();
}

/// Example-only root render object: registers its subtree as a repaint boundary when mounted and hands
/// the boundary's paint scope to a callback, so the driver can compose the window from it and mark it
/// dirty when the window is resized. In a real runtime this is the job of a "view" widget.
struct RootBoundary {
    content: BoundaryContent,
    layer: LayerHandle<ContainerLayer>,
    scope: Option<PaintScope>,
    on_ready: Option<Box<dyn FnOnce(PaintScope)>>,
}

impl RootBoundary {
    fn new(child: impl RenderBox, on_ready: impl FnOnce(PaintScope) + 'static) -> Self {
        Self {
            content: Rc::new(RefCell::new(Box::new(child) as Box<dyn AnyRenderBox>)),
            layer: LayerHandle::new(ContainerLayer::new()),
            scope: None,
            on_ready: Some(Box::new(on_ready)),
        }
    }
}

impl RenderObject for RootBoundary {
    fn mount(&mut self, ctx: &mut MountCtx) {
        let scope = ctx.register_boundary(Rc::clone(&self.content), self.layer.clone());

        if let Some(on_ready) = self.on_ready.take() {
            on_ready(scope.clone());
        }

        // Descendants repaint into this boundary.
        let content = Rc::clone(&self.content);
        ctx.with_paint_scope(scope.clone(), |ctx| content.borrow_mut().mount(ctx));

        self.scope = Some(scope);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.content.borrow_mut().unmount(ctx);

        if let Some(scope) = self.scope.take() {
            ctx.unregister_boundary(scope);
        }
    }
}

impl RenderBox for RootBoundary {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.content.borrow().max_intrinsic_height(width)
    }

    fn measure(&self, constraints: Constraints) -> Size {
        self.content.borrow().measure(constraints)
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        self.content.borrow_mut().layout(constraints)
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.content
            .borrow()
            .measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.content.borrow_mut().distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.content.borrow().hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx) {
        // Unused for the window root (nothing embeds it), but a boundary contributes its retained layer.
        ctx.add_layer(self.layer.clone().into());
    }
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
    /// Repaints dirty boundaries and composes the window's root into a scene.
    owner: RepaintOwner,
    /// The root render object, laid out each frame.
    render: RootBoundary,
    /// The window's root boundary; composed each frame and marked dirty on resize.
    root_scope: PaintScope,
    vello_scene: vello::Scene,
}

impl App {
    fn new(owner: RepaintOwner, render: RootBoundary, root_scope: PaintScope) -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
            active: None,
            owner,
            render,
            root_scope,
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

        self.active = Some(ActiveWindow { surface, window });
        tracing::info!(width = size.width, height = size.height, "window created");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(active) = self.active.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                tracing::info!(
                    width = size.width,
                    height = size.height,
                    "resized; marking root"
                );
                self.context
                    .resize_surface(&mut active.surface, size.width, size.height);
                // The layout changed, so the root boundary must repaint at the new size.
                self.root_scope.mark_needs_paint();
                active.window.request_redraw();
            }

            WindowEvent::RedrawRequested => {
                let width = active.surface.config.width;
                let height = active.surface.config.height;

                let _frame = tracing::info_span!("frame", width, height).entered();

                self.render
                    .layout(Constraints::new(0.0, width as f32, 0.0, height as f32));

                self.owner.flush_paint();
                let scene = self.owner.compose(&self.root_scope);

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
}
