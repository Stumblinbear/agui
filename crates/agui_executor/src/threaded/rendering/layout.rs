use std::hash::BuildHasherDefault;

use agui_core::{
    element::{deferred::resolver::DeferredResolver, ElementId},
    engine::rendering::{
        context::RenderingLayoutContext, scheduler::RenderingSchedulerStrategy,
        strategies::RenderingTreeLayoutStrategy, RenderingTree,
    },
    render::{object::RenderObject, RenderObjectId},
};
use futures::prelude::sink::SinkExt;
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::threaded::resolve_deferred::ResolveDeferredElement;

pub struct ThreadedLayoutRenderingTree<'layout, Sched> {
    pub scheduler: &'layout mut Sched,

    pub deferred_elements: &'layout mut SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    pub needs_paint: &'layout mut FxHashSet<RenderObjectId>,

    pub resolve_deferred_tx: &'layout mut futures::channel::mpsc::Sender<ResolveDeferredElement>,
}

impl<Sched> RenderingTreeLayoutStrategy for ThreadedLayoutRenderingTree<'_, Sched>
where
    Sched: RenderingSchedulerStrategy,
{
    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn on_constraints_changed(
        &mut self,
        ctx: RenderingLayoutContext,
        render_object: &RenderObject,
    ) {
        if let Some((deferred_element_id, resolver)) =
            self.deferred_elements.get_mut(*ctx.render_object_id)
        {
            tracing::trace!(
                render_object_id = ?ctx.render_object_id,
                element_id = ?deferred_element_id,
                "deferred element constraints changed, checking resolver",
            );

            if resolver.resolve(
                render_object
                    .constraints()
                    .expect("no constraints set for deferred render object"),
            ) {
                tracing::debug!("deferred resolver indicated a change, rebuilding subtree");

                // This whole thing is a dirty hack to get the element tree access to the
                // rendering tree in the other thread without using mutexes for every layout
                // operation.
                //
                // Unfortunately this whole setup means deferred elements cause enormous layout
                // slowdowns when they change.
                let mut rendering_tree = RenderingTree::default();
                let mut deferred_elements = SparseSecondaryMap::default();

                std::mem::swap(ctx.tree, &mut rendering_tree);
                std::mem::swap(self.deferred_elements, &mut deferred_elements);

                let (reply_tx, reply_rx) = oneshot::channel();

                futures::executor::block_on(self.resolve_deferred_tx.send(
                    ResolveDeferredElement {
                        rendering_tree,

                        deferred_elements,

                        render_object_id: *ctx.render_object_id,

                        reply_tx,
                    },
                ))
                .expect("failed to send deferred element resolve request");

                let mut reply = reply_rx
                    .recv()
                    .expect("failed to receive deferred element resolve reply");

                std::mem::swap(ctx.tree, &mut reply.rendering_tree);
                std::mem::swap(self.deferred_elements, &mut reply.deferred_elements);

                self.needs_paint.extend(reply.needs_paint);
            } else {
                // No need to do anything, since the resolver has indicated no change.
            }
        } else {
            tracing::trace!(
                render_object_id = ?ctx.render_object_id,
                constraints = ?render_object.constraints(),
                "constraints changed",
            );
        }
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn on_size_changed(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {
        if render_object.does_paint() {
            self.needs_paint.insert(*ctx.render_object_id);
        }

        tracing::trace!(
            render_object_id = ?ctx.render_object_id,
            size = ?render_object.size(),
            "size changed",
        );
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn on_offset_changed(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {
        tracing::trace!(
            render_object_id = ?ctx.render_object_id,
            offset = ?render_object.offset(),
            "offset changed",
        );
    }
}
