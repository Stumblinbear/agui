use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use agui_core::tree::{NodeHandle, Tree};

use crate::{
    context::{CreateCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{AnyElement, Element},
    pipeline::build_tree::{Build, BuildQueue, Operation, run},
    pipeline::render_pipeline::RenderPipeline,
    provide::ProvideScope,
    render_object::box_layout::AnyRenderBox,
    scheduling::TaskScheduler,
    widget::Widget,
};

/// The root element's boxed form. The root renders nothing of its own; a tree's render subtrees are planted
/// by the [`View`](crate::view::View)s it holds.
type RootElement = Box<dyn AnyElement<Render = ()>>;

pub mod build_tree;
pub mod render_pipeline;

mod phase;

pub(crate) use phase::FramePhase;

/// A render object shared between the layout and paint registries, so a node that is both a relayout
/// and a repaint boundary is held in one place.
pub type BoundaryContent = Rc<RefCell<dyn AnyRenderBox>>;

/// Drives one widget tree: it builds and rebuilds the element tree, and lays out and paints the render
/// boundaries the tree's [`View`](crate::view::View)s plant in its [`RenderPipeline`]. The root renders
/// nothing; each `View` retains and presents its own render subtree through the pipeline.
pub struct PipelineOwner {
    tree: Tree<RootElement, Build>,
    queue: BuildQueue,
    provide: ProvideScope,
    pipeline: RenderPipeline,
}

impl PipelineOwner {
    /// Builds `widget`'s element as the root of a fresh tree and mounts it. The root widget renders nothing;
    /// a `View` in the tree plants its render tree in the pipeline during `create`, and a
    /// [`ViewHandle`](crate::view::ViewHandle) retains it.
    pub fn new<V>(widget: V, scheduler: &mut dyn TaskScheduler) -> Self
    where
        V: Widget<Render = ()> + 'static,
        V::Element: 'static,
    {
        let provide = ProvideScope::default();
        let mut queue = BuildQueue::new();
        let pipeline = RenderPipeline::default();

        let element = widget.create(&mut CreateCtx::new(provide, pipeline.clone()));
        let root: RootElement = Box::new(element);

        let tree = Tree::<RootElement, Build>::new(root, run::<RootElement>, |root, cursor| {
            root.mount(&mut UpdateCtx::new(
                cursor, provide, &mut queue, &pipeline, scheduler,
            ));
        });

        Self {
            tree,
            queue,
            provide,
            pipeline,
        }
    }

    /// Registers `f` to fire when the pipeline goes from idle to having pending layout or paint work, so a
    /// driver schedules a frame. A frame runs `flush_build` + `flush_layout` + `flush_paint`; the dirty
    /// channels decide what reruns, so scheduling is idempotent.
    pub fn on_needs_frame(&self, f: Box<dyn Fn()>) {
        self.pipeline.on_needs_frame(f);
    }

    /// Whether any element is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Rebuilds every element marked since the last flush, shallowest first. An element marked during the
    /// flush is honored in the same pass.
    pub fn flush_build(&mut self, scheduler: &mut dyn TaskScheduler) {
        let Self {
            tree,
            queue,
            provide,
            pipeline,
        } = self;
        let pipeline: &RenderPipeline = pipeline;

        // A reconcile marks layout and paint, which the pipeline accepts as pre-layout work.
        let _phase = pipeline.enter_phase(FramePhase::Build);

        while let Some((handle, is_dependency_change)) = queue.take_shallowest(tree) {
            tree.dispatch_with_cursor(handle, |cursor| {
                let ctx = UpdateCtx::new(cursor, *provide, queue, pipeline, scheduler);
                if is_dependency_change {
                    Operation::DependencyChanged(ctx)
                } else {
                    Operation::Rebuild(ctx)
                }
            });
        }
    }

    /// Re-lays every relayout boundary marked since the last frame, applying any out-of-band marks first.
    pub fn flush_layout(&self) {
        self.pipeline.drain_deferred();
        self.pipeline.flush_layout();
    }

    /// Repaints every repaint boundary marked since the last frame.
    pub fn flush_paint(&self) {
        self.pipeline.flush_paint();
    }

    /// Delivers `message` to the element at `handle`. If it requests a rebuild, it marks itself for the next
    /// [`flush_build`](Self::flush_build); the owner schedules a frame when that mark takes the build from
    /// clean to dirty.
    pub fn dispatch_message(&mut self, handle: NodeHandle, message: Box<dyn Any>) {
        let was_clean = self.queue.is_empty();

        {
            let Self { tree, queue, .. } = self;
            tree.dispatch(
                handle,
                Operation::Message(MessageCtx::new(message, handle, queue)),
            );
        }

        if was_clean && !self.queue.is_empty() {
            self.pipeline.request_frame();
        }
    }

    /// Captures the element tree as a diagnostics snapshot.
    pub fn diagnostics(&self) -> DiagnosticsNode {
        self.tree.root().describe(&mut Diagnostics::new())
    }
}
