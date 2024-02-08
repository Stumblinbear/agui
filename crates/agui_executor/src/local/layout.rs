use std::{collections::VecDeque, hash::BuildHasherDefault, sync::Arc};

use agui_core::{
    callback::strategies::CallbackStrategy,
    element::{
        deferred::resolver::DeferredResolver, Element, ElementBuildContext, ElementId,
        ElementMountContext, ElementUnmountContext, RenderObjectCreateContext,
        RenderObjectUpdateContext,
    },
    engine::{
        elements::{
            context::{ElementTreeContext, ElementTreeMountContext},
            scheduler::ElementSchedulerStrategy,
            strategies::{InflateElementStrategy, UnmountElementStrategy},
            ElementTree,
        },
        rendering::{
            context::{RenderingLayoutContext, RenderingSpawnContext, RenderingUpdateContext},
            strategies::{
                RenderingTreeCreateStrategy, RenderingTreeLayoutStrategy,
                RenderingTreeUpdateStrategy,
            },
            view::View,
            RenderingTree,
        },
    },
    render::{object::RenderObject, RenderObjectId},
    widget::Widget,
};
use rustc_hash::{FxHashSet, FxHasher};
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::local::{rendering_cleanup::RenderingTreeCleanup, scheduler::LocalScheduler};

pub struct LayoutRenderObjects<'layout> {
    pub scheduler: &'layout mut LocalScheduler,
    pub callbacks: &'layout Arc<dyn CallbackStrategy>,

    pub element_tree: &'layout mut ElementTree,

    pub deferred_elements:
        &'layout mut SecondaryMap<RenderObjectId, (ElementId, Box<dyn DeferredResolver>)>,

    pub needs_paint: &'layout mut FxHashSet<RenderObjectId>,
}

impl RenderingTreeLayoutStrategy for LayoutRenderObjects<'_> {
    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn on_constraints_changed(
        &mut self,
        ctx: RenderingLayoutContext,
        render_object: &RenderObject,
    ) {
        struct DeferredRebuildStrategy<'rebuild, Sched> {
            scheduler: &'rebuild mut Sched,
            callbacks: &'rebuild Arc<dyn CallbackStrategy>,

            spawned_elements: &'rebuild mut VecDeque<ElementId>,
            updated_elements:
                &'rebuild mut SparseSecondaryMap<ElementId, (), BuildHasherDefault<FxHasher>>,

            rebuilt_elements: &'rebuild mut FxHashSet<ElementId>,
        }

        impl<Sched> InflateElementStrategy for DeferredRebuildStrategy<'_, Sched>
        where
            Sched: ElementSchedulerStrategy,
        {
            type Definition = Widget;

            #[tracing::instrument(level = "debug", skip(self, ctx))]
            fn mount(
                &mut self,
                ctx: ElementTreeMountContext,
                definition: Self::Definition,
            ) -> Element {
                self.spawned_elements.push_back(*ctx.element_id);

                let mut element = definition.create_element();

                element.mount(&mut ElementMountContext {
                    element_tree: ctx.tree,

                    parent_element_id: ctx.parent_element_id,
                    element_id: ctx.element_id,
                });

                element
            }

            #[tracing::instrument(level = "debug", skip(self))]
            fn try_update(
                &mut self,
                id: ElementId,
                element: &mut Element,
                definition: &Self::Definition,
            ) -> agui_core::element::ElementComparison {
                self.updated_elements.insert(id, ());

                element.update(definition)
            }

            #[tracing::instrument(level = "debug", skip(self, ctx))]
            fn build(&mut self, ctx: ElementTreeContext, element: &mut Element) -> Vec<Widget> {
                self.rebuilt_elements.insert(*ctx.element_id);

                element.build(&mut ElementBuildContext {
                    scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),
                    callbacks: self.callbacks,

                    element_tree: ctx.tree,
                    inheritance: ctx.inheritance,

                    element_id: ctx.element_id,
                })
            }
        }

        struct DeferredCreateRenderObjectStrategy<'create> {
            scheduler: &'create mut LocalScheduler,

            element_tree: &'create ElementTree,
            deferred_elements:
                &'create mut SecondaryMap<RenderObjectId, (ElementId, Box<dyn DeferredResolver>)>,

            needs_paint: &'create mut FxHashSet<RenderObjectId>,
        }

        impl RenderingTreeCreateStrategy for DeferredCreateRenderObjectStrategy<'_> {
            #[tracing::instrument(level = "debug", skip(self, ctx))]
            fn create(
                &mut self,
                ctx: RenderingSpawnContext,
                element_id: ElementId,
            ) -> RenderObject {
                let element = self
                    .element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while creating render object");

                if let Element::Deferred(element) = element {
                    self.deferred_elements.insert(
                        *ctx.render_object_id,
                        (element_id, element.create_resolver()),
                    );
                }

                let render_object = self
                    .element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while creating render object")
                    .create_render_object(&mut RenderObjectCreateContext {
                        scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                        render_object_id: ctx.render_object_id,
                    });

                // We shouldn't need to mark needs_layout here, since the deferred element is
                // already in the middle of a layout pass.

                if render_object.does_paint() {
                    self.needs_paint.insert(*ctx.render_object_id);
                }

                render_object
            }

            #[tracing::instrument(level = "debug", skip(self))]
            fn create_view(&mut self, element_id: ElementId) -> Option<Box<dyn View + Send>> {
                if let Element::View(view) = self
                    .element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while creating view")
                {
                    Some(view.create_view())
                } else {
                    None
                }
            }
        }

        struct SyncUpdateRenderObjectStrategy<'update> {
            scheduler: &'update mut LocalScheduler,

            element_tree: &'update ElementTree,

            needs_paint: &'update mut FxHashSet<RenderObjectId>,
        }

        impl RenderingTreeUpdateStrategy for SyncUpdateRenderObjectStrategy<'_> {
            fn get_children(&self, element_id: ElementId) -> &[ElementId] {
                self.element_tree
                    .as_ref()
                    .get_children(element_id)
                    .expect("element missing while updating render object")
            }

            #[tracing::instrument(level = "debug", skip(self, ctx))]
            fn update(
                &mut self,
                ctx: RenderingUpdateContext,
                element_id: ElementId,
                render_object: &mut RenderObject,
            ) {
                let mut needs_paint = false;

                self.element_tree
                    .as_ref()
                    .get(element_id)
                    .expect("element missing while updating render object")
                    .update_render_object(
                        &mut RenderObjectUpdateContext {
                            scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                            needs_layout: &mut false,
                            needs_paint: &mut needs_paint,

                            render_object_id: ctx.render_object_id,
                        },
                        render_object,
                    );

                if needs_paint {
                    self.needs_paint.insert(*ctx.render_object_id);
                }
            }
        }

        struct DeferredUnmountedStrategy<'cleanup> {
            rendering_tree: &'cleanup mut RenderingTree,

            updated_elements:
                &'cleanup mut SparseSecondaryMap<ElementId, (), BuildHasherDefault<FxHasher>>,
        }

        impl UnmountElementStrategy for DeferredUnmountedStrategy<'_> {
            fn unmount(&mut self, mut ctx: ElementUnmountContext, element: Element) {
                self.rendering_tree.forget(*ctx.element_id);

                self.updated_elements.remove(*ctx.element_id);

                element.unmount(&mut ctx);
            }
        }

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

                let mut spawned_elements = VecDeque::new();
                let mut updated_elements = SparseSecondaryMap::default();
                let mut rebuilt_elements = FxHashSet::default();

                self.element_tree
                    .resolve_deferred(
                        &mut DeferredRebuildStrategy {
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
                    .cleanup(&mut DeferredUnmountedStrategy {
                        rendering_tree: ctx.tree,

                        updated_elements: &mut updated_elements,
                    })
                    .expect("failed to cleanup element tree");

                for element_id in spawned_elements.drain(..) {
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
                        &mut SyncUpdateRenderObjectStrategy {
                            scheduler: self.scheduler,

                            element_tree: self.element_tree,

                            needs_paint: self.needs_paint,
                        },
                        element_id,
                    );
                }

                ctx.tree
                    .cleanup(&mut RenderingTreeCleanup {
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
