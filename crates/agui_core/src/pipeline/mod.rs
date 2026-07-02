use std::any::Any;

use crate::context::{CreateCtx, MessageCtx, UpdateCtx};
use crate::diagnostics::{Diagnostics, DiagnosticsNode};
use crate::element::{AnyElement, Element};
use crate::pipeline::build_tree::{Build, Operation, run};
use crate::pipeline::render_pipeline::RenderPipeline;
use crate::provide::ProvideScope;
use crate::scheduling::TaskScheduler;
use crate::tree::{NodeHandle, Tree};
use crate::{build_queue::BuildQueue, pipeline::render_pipeline::LayoutBuildHost};

pub mod build_tree;
pub mod render_pipeline;

mod phase;

pub(crate) use phase::FramePhase;

/// The root element's boxed form. The root renders nothing of its own; a tree's render subtrees are planted by
/// the views it holds.
pub type RootElement = Box<dyn AnyElement<Render = ()>>;

/// Drives one widget tree: it builds and rebuilds the element tree, and lays out and paints the render
/// boundaries the tree's views plant in its [`RenderPipeline`]. The root renders nothing; each view retains
/// and presents its own render subtree through the pipeline.
pub struct PipelineOwner {
    tree: Tree<RootElement, Build>,
    queue: BuildQueue,
    provide: ProvideScope,
    pipeline: RenderPipeline,
}

impl PipelineOwner {
    /// Builds the root element with `create_root` and mounts it as the root of a fresh tree. The root renders
    /// nothing; a view in the tree plants its render tree in the pipeline during `create`, and a view handle
    /// retains it. `create_root` receives the [`CreateCtx`] carrying the pipeline the owner just built.
    pub fn new(
        create_root: impl FnOnce(&mut CreateCtx) -> RootElement,
        scheduler: &mut dyn TaskScheduler,
    ) -> Self {
        let provide = ProvideScope::default();
        let mut queue = BuildQueue::new();
        let pipeline = RenderPipeline::default();

        let root = create_root(&mut CreateCtx::new(
            provide,
            pipeline.layout(),
            pipeline.paint(),
            pipeline.semantics(),
        ));

        let tree = Tree::<RootElement, Build>::new(root, run::<RootElement>, |root, cursor| {
            root.mount(&mut UpdateCtx::new(
                cursor,
                provide,
                &mut queue,
                pipeline.layout(),
                pipeline.paint(),
                pipeline.semantics(),
                scheduler,
            ));
        });

        Self {
            tree,
            queue,
            provide,
            pipeline,
        }
    }

    /// Whether any element is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        !self.queue.is_empty()
    }

    /// Registers `f` to fire when a view's semantics change, so the driver re-reads them.
    pub fn on_needs_semantics_update(&self, f: Box<dyn Fn()>) {
        self.pipeline.on_needs_semantics_update(f);
    }

    pub fn flush(&mut self, scheduler: &mut dyn TaskScheduler) {
        self.pipeline.reset_notified();

        self.flush_build(scheduler);
        self.flush_layout();
        self.flush_paint();
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
                let ctx = UpdateCtx::new(
                    cursor,
                    *provide,
                    queue,
                    pipeline.layout(),
                    pipeline.paint(),
                    pipeline.semantics(),
                    scheduler,
                );

                if is_dependency_change {
                    Operation::DependencyChanged(ctx)
                } else {
                    Operation::Rebuild(ctx)
                }
            });
        }
    }

    /// Re-lays every relayout boundary marked since the last frame, applying any out-of-band marks first.
    pub fn flush_layout(&mut self) {
        let Self {
            tree,
            queue,
            provide,
            pipeline,
        } = self;
        let pipeline = pipeline.clone();

        let _phase = self.pipeline.enter_phase(FramePhase::Layout);

        let mut host = LayoutBuildHost::new(tree, queue, *provide, &pipeline);

        pipeline.flush_layout(&mut host);
    }

    /// Repaints every repaint boundary marked since the last frame.
    pub fn flush_paint(&self) {
        self.pipeline.flush_paint();
    }

    /// Re-walks each semantics boundary marked since the last frame, delivering each view's freshly built
    /// semantics to that view's sink, then re-arms so the next change fires the callback again.
    pub fn flush_semantics(&self) {
        self.pipeline.flush_semantics();
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
