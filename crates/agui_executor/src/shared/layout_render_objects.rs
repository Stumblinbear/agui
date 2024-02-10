use std::{hash::BuildHasherDefault, sync::Arc};

use agui_core::{
    callback::strategies::CallbackStrategy,
    element::{deferred::resolver::DeferredResolver, ElementId},
    engine::{
        elements::{scheduler::ElementSchedulerStrategy, ElementTree},
        rendering::{
            context::RenderingLayoutContext, scheduler::RenderingSchedulerStrategy,
            strategies::RenderingTreeLayoutStrategy,
        },
    },
    render::{object::RenderObject, RenderObjectId},
};
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::shared::{
    cleanup_rendering_tree::CleanupRenderingTree,
    deferred::{
        create_render_object::DeferredCreateRenderObjectStrategy,
        update_render_object::DeferredUpdateRenderObjectStrategy,
    },
    rebuild::RebuildStrategy,
    unmount::ElementTreeUnmount,
};

pub struct LayoutRenderingTree<'layout, Sched> {
    pub scheduler: &'layout mut Sched,

    pub callbacks: &'layout Arc<dyn CallbackStrategy>,

    pub element_tree: &'layout mut ElementTree,

    pub deferred_elements: &'layout mut SparseSecondaryMap<
        RenderObjectId,
        (ElementId, Box<dyn DeferredResolver>),
        BuildHasherDefault<FxHasher>,
    >,

    pub needs_paint: &'layout mut FxHashSet<RenderObjectId>,
}

impl<Sched> RenderingTreeLayoutStrategy for LayoutRenderingTree<'_, Sched>
where
    Sched: ElementSchedulerStrategy + RenderingSchedulerStrategy,
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

                let mut spawned_elements = Vec::new();
                let mut updated_elements = SparseSecondaryMap::default();
                let mut rebuilt_elements = FxHashSet::default();

                self.element_tree
                    .resolve_deferred(
                        &mut RebuildStrategy {
                            scheduler: self.scheduler,
                            callbacks: self.callbacks,

                            spawned_elements: &mut spawned_elements,
                            updated_elements: &mut updated_elements,

                            rebuilt_elements: &mut rebuilt_elements,
                        },
                        *deferred_element_id,
                        resolver.as_ref(),
                    )
                    .expect("failed to build deferred element subtree");

                self.element_tree
                    .cleanup(&mut ElementTreeUnmount {
                        rendering_tree: ctx.tree,

                        updated_elements: &mut updated_elements,
                    })
                    .expect("failed to cleanup element tree");

                for element_id in spawned_elements {
                    ctx.tree.create(
                        &mut DeferredCreateRenderObjectStrategy {
                            scheduler: self.scheduler,

                            element_tree: self.element_tree,
                            deferred_elements: self.deferred_elements,

                            needs_paint: self.needs_paint,
                        },
                        self.element_tree.as_ref().get_parent(element_id).copied(),
                        element_id,
                    );

                    updated_elements.remove(element_id);
                }

                for element_id in updated_elements.drain().map(|(id, _)| id) {
                    ctx.tree.update(
                        &mut DeferredUpdateRenderObjectStrategy {
                            scheduler: self.scheduler,

                            element_tree: self.element_tree,

                            needs_paint: self.needs_paint,
                        },
                        element_id,
                    );
                }

                ctx.tree
                    .cleanup(&mut CleanupRenderingTree {
                        deferred_elements: self.deferred_elements,
                    })
                    .expect("failed to cleanup rendering tree");
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
