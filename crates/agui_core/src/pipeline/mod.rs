use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    context::{MessageCtx, MountCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::RoutingTarget,
    pipeline::{
        build::{BuildBoundaryElement, BuildBoundaryId, BuildState},
        layout::LayoutPipeline,
        paint::{PaintPipeline, PaintScope},
    },
    prelude::render_object::AnyRenderBox,
    provide::ProvideScope,
    render_object::RenderObject,
    scheduling::TaskScheduler,
    widget::Widget,
};

pub mod boundary;
pub mod build;
pub mod layout;
pub mod paint;

mod phase;

pub(crate) use phase::{FramePhase, enter_phase};

/// A render object shared between the layout and paint registries, so a node that is both a relayout
/// and a repaint boundary is held in one place.
pub type BoundaryContent = Rc<RefCell<dyn AnyRenderBox>>;

/// Drives one widget tree's build and pipeline state: it builds the element tree from a root widget,
/// rebuilds the boundaries that ask to, and flushes the layout and paint of every boundary registered
/// under it.
///
/// It holds no render root of its own. A render subtree presented to a surface is established by a
/// [`View`](crate::view::View) inside the tree, which self-registers its relayout and repaint boundary
/// during mount and surfaces a [`ViewHandle`](crate::view::ViewHandle) for the per-view operations. The
/// owner's pipelines lay out and paint whatever boundaries those views and the inner repaint boundaries
/// register, so a tree may hold any number of views, or none.
pub struct PipelineOwner {
    build_state: Rc<RefCell<BuildState>>,
    build_root: BuildBoundaryElement,

    layout: LayoutPipeline,
    paint: PaintPipeline,
}

impl PipelineOwner {
    /// Builds `widget` as the root of a fresh tree, registering its element as the outermost build
    /// boundary and mounting its render subtree. Any [`View`](crate::view::View) the tree holds
    /// self-registers as it mounts.
    pub fn new<V>(widget: V, scheduler: &mut dyn TaskScheduler) -> Self
    where
        V: Widget,
        V::Element: 'static,
        V::Render: RenderObject,
    {
        let provide = ProvideScope::new();
        let (build_state, root_scope) = BuildState::new();

        let layout = LayoutPipeline::default();
        let mut paint = PaintPipeline::default();
        let detached = PaintScope::detached();

        let (build_root, mut render) = {
            let mut path = Vec::new();

            // `create` builds without mounting, so it registers nothing; the live pipelines it threads
            // are first touched by the mount below.
            let mut ctx = UpdateCtx::new(
                scheduler,
                &mut path,
                &provide,
                &root_scope,
                &layout,
                &mut paint,
                &detached,
            );

            BuildBoundaryElement::create(widget, &mut ctx)
        };

        {
            let mut ctx = MountCtx::new(&layout, &mut paint, &detached);
            render.mount(&mut ctx);
        }

        Self {
            build_state,
            build_root,
            layout,
            paint,
        }
    }

    pub fn on_needs_layout(&mut self, f: Box<dyn Fn()>) {
        self.layout.on_needs_layout(f);
    }

    pub fn on_needs_paint(&mut self, f: Box<dyn Fn()>) {
        self.paint.on_needs_paint(f);
    }

    /// The id of the root build boundary, for addressing a root-relative path.
    pub fn root_id(&self) -> BuildBoundaryId {
        self.build_root.id()
    }

    /// Whether any build boundary is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        self.build_state.borrow().is_dirty()
    }

    /// Delivers `message` to the element at `path`. If that element asks to rebuild, its boundary is
    /// marked for the next [`flush_build`](Self::flush_build).
    pub fn dispatch_message(&mut self, target: &RoutingTarget, message: Box<dyn Any>) {
        let mut ctx = MessageCtx::new(message);

        BuildState::deliver_message(&self.build_state, target, &mut ctx);
    }

    /// Rebuilds every build boundary marked since the last flush. Returns whether anything rebuilt, so
    /// the caller can skip reconciling the render tree when nothing changed. A render object built during
    /// a rebuild is mounted under the boundary enclosing it.
    pub fn flush_build(&mut self, scheduler: &mut dyn TaskScheduler) -> bool {
        BuildState::flush(&self.build_state, scheduler, &self.layout, &mut self.paint)
    }

    /// Lays out any boundary that has been marked for layout since the last flush.
    pub fn flush_layout(&mut self) {
        self.layout.drain_deferred();

        self.layout.flush(&mut self.paint);
    }

    /// Repaints any boundary that has been marked for paint since the last flush.
    pub fn flush_paint(&mut self) {
        self.paint.drain_deferred();

        self.paint.flush();
    }

    /// Captures the element tree under the root boundary as a diagnostics snapshot.
    pub fn diagnostics(&self) -> DiagnosticsNode {
        self.build_root.describe(&mut Diagnostics::new())
    }
}
