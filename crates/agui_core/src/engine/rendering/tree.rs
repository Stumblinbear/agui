use core::panic;
use std::hash::BuildHasherDefault;

use rustc_hash::{FxHashSet, FxHasher};
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    element::ElementId,
    engine::rendering::{
        context::{RenderingLayoutContext, RenderingSpawnContext, RenderingUpdateContext},
        errors::RemoveError,
        scheduler::RenderingScheduler,
        strategies::{
            RenderingTreeCleanupStrategy, RenderingTreeCreateStrategy, RenderingTreeLayoutStrategy,
            RenderingTreeUpdateStrategy,
        },
        view::View,
        RenderViews,
    },
    render::{
        object::{RenderObject, RenderObjectLayoutContext},
        RenderObjectId,
    },
    util::tree::Tree,
};

#[derive(Default)]
pub struct RenderingTree {
    tree: Tree<RenderObjectId, RenderObject>,

    element_mapping: SecondaryMap<ElementId, RenderObjectId>,

    forgotten_elements: SparseSecondaryMap<RenderObjectId, (), BuildHasherDefault<FxHasher>>,

    render_views: RenderViews,
}

impl RenderingTree {
    pub fn get(&self, id: RenderObjectId) -> Option<&RenderObject> {
        self.tree.get(id)
    }

    pub fn with<F, R>(&mut self, id: RenderObjectId, func: F) -> Option<R>
    where
        F: FnOnce(&mut Self, &mut RenderObject) -> R,
    {
        if let Some(mut value) = self.tree.take(id) {
            let ret = func(self, &mut value);

            self.tree.replace(id, value);

            Some(ret)
        } else {
            None
        }
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn forget(&mut self, element_id: ElementId) {
        if let Some(render_object_id) = self.element_mapping.remove(element_id) {
            self.forgotten_elements.insert(render_object_id, ());
        }
    }

    pub fn create<S>(
        &mut self,
        strategy: &mut S,
        parent_element_id: Option<ElementId>,
        element_id: ElementId,
    ) -> RenderObjectId
    where
        S: RenderingTreeCreateStrategy,
    {
        if let Some(render_object_id) = self.element_mapping.get(element_id) {
            panic!(
                "element already has a render object: {:?}",
                render_object_id
            );
        }

        let parent_render_object_id = parent_element_id.map(|parent_element_id| {
            self.element_mapping
                .get(parent_element_id)
                .copied()
                .expect("parent element has no render object while creating render object")
        });

        let parent_view_id = parent_render_object_id.and_then(|parent_render_object_id| {
            self.render_views.get_owner_id(parent_render_object_id)
        });

        self.tree
            .add_with_key(parent_render_object_id, |_, render_object_id| {
                self.element_mapping.insert(element_id, render_object_id);

                let render_object = strategy.create(
                    RenderingSpawnContext {
                        scheduler: RenderingScheduler::new(&render_object_id),

                        parent_render_object_id: &parent_render_object_id,
                        render_object_id: &render_object_id,
                    },
                    element_id,
                );

                if let Some(mut render_view) = strategy.create_view(element_id) {
                    // Attach the render object as the root of its own view
                    render_view.on_attach(None, render_object_id);

                    self.render_views.create_view(render_object_id, render_view);
                } else if let Some(parent_view_id) = parent_view_id {
                    let view = self
                        .render_views
                        .get_mut(parent_view_id)
                        .expect("parent render object has no view while creating render objects");

                    view.on_attach(parent_render_object_id, render_object_id);

                    self.render_views
                        .set_within_view(render_object_id, parent_view_id);
                };

                render_object
            })
    }

    #[tracing::instrument(level = "trace", skip_all)]
    pub fn update<S>(&mut self, strategy: &mut S, element_id: ElementId)
    where
        S: RenderingTreeUpdateStrategy,
    {
        let render_object_id = self
            .element_mapping
            .get(element_id)
            .copied()
            .expect("element has no render object to update");

        let render_object = self
            .tree
            .get_mut(render_object_id)
            .expect("render object missing while updating");

        strategy.update(
            RenderingUpdateContext {
                scheduler: RenderingScheduler::new(&render_object_id),

                render_object_id: &render_object_id,
            },
            element_id,
            render_object,
        );

        // Sync the order of the render objects of the element's children. We've already
        // created/removed all necessary render objects, so we just need to make sure
        // that the order of the render objects matches the order of the element's children.
        self.tree
            .reorder_children(
                render_object_id,
                strategy
                    .get_children(element_id)
                    .iter()
                    .copied()
                    .map(|element_id| {
                        *self.element_mapping.get(element_id).expect(
                            "child element has no render object while syncing render object",
                        )
                    })
                    .collect(),
            )
            .expect("failed to reorder render object children");
    }

    pub fn cleanup(
        &mut self,
        strategy: &mut dyn RenderingTreeCleanupStrategy,
    ) -> Result<(), Vec<RemoveError<RenderObjectId>>> {
        let subtree_roots = self
            .forgotten_elements
            .drain()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();

        let mut failed_to_unmount = Vec::new();

        let mut views_to_remove = Vec::new();

        for render_object_id in &subtree_roots {
            tracing::trace!(?render_object_id, "removing from the tree");

            let Some(node) = self.tree.get_node_mut(*render_object_id) else {
                continue;
            };

            match node.take() {
                Ok(value) => value,
                Err(_) => {
                    failed_to_unmount.push(RemoveError::InUse(*render_object_id));
                    continue;
                }
            };

            if self.render_views.is_owner(*render_object_id) {
                views_to_remove.push(*render_object_id);
            } else {
                self.render_views.remove_within(*render_object_id);
            }

            strategy.on_removed(*render_object_id);
        }

        for render_object_id in subtree_roots {
            self.tree.remove_subtree(render_object_id);
        }

        for render_object_id in views_to_remove {
            self.render_views.remove_view(render_object_id);
        }

        if failed_to_unmount.is_empty() {
            Ok(())
        } else {
            Err(failed_to_unmount)
        }
    }

    pub fn layout<S>(
        &mut self,
        strategy: &mut S,
        needs_layout: impl IntoIterator<Item = RenderObjectId>,
    ) where
        S: RenderingTreeLayoutStrategy,
    {
        struct LayoutStrategy<'layout, S> {
            inner: &'layout mut S,

            laid_out: &'layout mut FxHashSet<RenderObjectId>,
        }

        impl<S> RenderingTreeLayoutStrategy for LayoutStrategy<'_, S>
        where
            S: RenderingTreeLayoutStrategy,
        {
            fn on_constraints_changed(
                &mut self,
                ctx: RenderingLayoutContext,
                render_object: &RenderObject,
            ) {
                self.inner.on_constraints_changed(ctx, render_object);
            }

            fn on_size_changed(
                &mut self,
                ctx: RenderingLayoutContext,
                render_object: &RenderObject,
            ) {
                if let Some(view) = ctx.tree.render_views.get_mut(*ctx.render_object_id) {
                    view.on_size_changed(*ctx.render_object_id, render_object.size());

                    ctx.tree.render_views.mark_needs_sync(*ctx.render_object_id);
                }

                self.inner.on_size_changed(ctx, render_object);
            }

            fn on_offset_changed(
                &mut self,
                ctx: RenderingLayoutContext,
                render_object: &RenderObject,
            ) {
                if let Some(view) = ctx.tree.render_views.get_mut(*ctx.render_object_id) {
                    view.on_offset_changed(*ctx.render_object_id, render_object.offset());

                    ctx.tree.render_views.mark_needs_sync(*ctx.render_object_id);
                }

                self.inner.on_offset_changed(ctx, render_object);
            }

            fn on_laid_out(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {
                self.laid_out.insert(*ctx.render_object_id);

                self.inner.on_laid_out(ctx, render_object);
            }
        }

        let mut laid_out = FxHashSet::default();

        let mut needs_layout = needs_layout
            .into_iter()
            .map(|render_object_id| {
                self.tree
                    .get(render_object_id)
                    .unwrap()
                    .relayout_boundary_id()
                    .unwrap_or(render_object_id)
            })
            // Is it worth collecting into a HashSet first to filter out duplicate boundaries?
            // .collect::<FxHashSet<_>>()
            // .into_iter()
            .collect::<Vec<_>>();

        needs_layout
            .sort_by_cached_key(|render_object_id| self.tree.get_depth(*render_object_id).unwrap());

        for id in needs_layout {
            tracing::trace!(?id, "laying out render object");

            // It's likely that a nested render object will have already been processed by
            // a previous iteration of the loop, so we can skip it here.
            if laid_out.contains(&id) {
                tracing::trace!(?id, "render object has already been laid out, skipping");

                continue;
            }

            let constraints = self
                .tree
                .get_parent(id)
                .and_then(|parent_id| {
                    self.tree
                        .get(*parent_id)
                        .expect("parent missing while fetching previous constraints during layout")
                        .constraints()
                })
                .unwrap_or_default();

            tracing::trace!(?id, ?constraints, "layout constraints");

            self.with(id, |tree, render_object| {
                // TODO: figure out how to not clone this
                let children = tree.as_ref().get_children(id).cloned().unwrap_or_default();

                render_object.layout(
                    &mut RenderObjectLayoutContext {
                        strategy: &mut LayoutStrategy {
                            inner: strategy,

                            laid_out: &mut laid_out,
                        },

                        tree,

                        parent_uses_size: &false,

                        relayout_boundary_id: &Some(id),

                        render_object_id: &id,

                        children: &children,
                    },
                    constraints,
                );
            })
            .expect("render object missing while laying out");
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn paint(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!(?render_object_id, "painting render object");

        let render_object = self
            .tree
            .get_mut(render_object_id)
            .expect("render object missing while flushing paint");

        let Some(view) = self.render_views.get_mut(render_object_id) else {
            return;
        };

        let canvas = render_object.paint();

        view.on_paint(render_object_id, canvas);

        self.render_views.mark_needs_sync(render_object_id);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn sync_views(&mut self) {
        self.render_views.sync();
    }

    pub(crate) fn get_view(&self, render_object_id: RenderObjectId) -> Option<&dyn View> {
        self.render_views.get(render_object_id)
    }

    pub(crate) fn get_view_mut(
        &mut self,
        render_object_id: RenderObjectId,
    ) -> Option<&mut dyn View> {
        self.render_views.get_mut(render_object_id)
    }
}

impl AsRef<Tree<RenderObjectId, RenderObject>> for RenderingTree {
    fn as_ref(&self) -> &Tree<RenderObjectId, RenderObject> {
        &self.tree
    }
}
