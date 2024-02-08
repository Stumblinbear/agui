use core::panic;
use std::hash::BuildHasherDefault;

use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
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
        view::RenderView,
    },
    render::{
        object::{RenderObject, RenderObjectLayoutContext},
        RenderObjectId,
    },
    unit::{Offset, Size},
    util::tree::Tree,
};

#[derive(Default)]
pub struct RenderingTree {
    tree: Tree<RenderObjectId, RenderObject>,

    element_mapping: SecondaryMap<ElementId, RenderObjectId>,

    forgotten_elements: SparseSecondaryMap<RenderObjectId, (), BuildHasherDefault<FxHasher>>,

    render_views: SparseSecondaryMap<RenderObjectId, RenderView, BuildHasherDefault<FxHasher>>,
    needs_sync: SparseSecondaryMap<RenderObjectId, (), BuildHasherDefault<FxHasher>>,
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

        let parent_view_id = parent_render_object_id.and_then(|parent_render_object_id| match self
            .render_views
            .get(parent_render_object_id)
        {
            Some(RenderView::Owner(_)) => Some(parent_render_object_id),
            Some(RenderView::Within(view_object_id)) => Some(*view_object_id),
            None => None,
        });

        self.tree
            .add_with_key(parent_render_object_id, |_, render_object_id| {
                self.element_mapping.insert(element_id, render_object_id);

                let render_object = strategy.create(
                    RenderingSpawnContext {
                        scheduler: RenderingScheduler::new(&render_object_id),

                        render_object_id: &render_object_id,
                    },
                    element_id,
                );

                if let Some(mut render_view) = strategy.create_view(element_id) {
                    // Attach the render object as the root of its own view
                    render_view.on_attach(None, render_object_id);

                    self.render_views
                        .insert(render_object_id, RenderView::Owner(render_view));
                } else if let Some(parent_view_id) = parent_view_id {
                    let view =
                        match self.render_views.get_mut(parent_view_id).expect(
                            "parent render object has no view while creating render objects",
                        ) {
                            RenderView::Owner(ref mut view) => view,
                            _ => panic!(
                            "parent render object is not a view owner while creating render objects"
                        ),
                        };

                    view.on_attach(parent_render_object_id, render_object_id);

                    self.render_views
                        .insert(render_object_id, RenderView::Within(parent_view_id));
                };

                self.needs_sync.insert(render_object_id, ());

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

            strategy.on_removed(*render_object_id);
        }

        for render_object_id in subtree_roots {
            self.tree.remove_subtree(render_object_id);
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

            changed: &'layout mut FxHashMap<RenderObjectId, (Option<Size>, Option<Offset>)>,

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
                self.changed.entry(*ctx.render_object_id).or_default().0 =
                    Some(render_object.size());

                self.inner.on_size_changed(ctx, render_object);
            }

            fn on_offset_changed(
                &mut self,
                ctx: RenderingLayoutContext,
                render_object: &RenderObject,
            ) {
                self.changed.entry(*ctx.render_object_id).or_default().1 =
                    Some(render_object.offset());

                self.inner.on_offset_changed(ctx, render_object);
            }

            fn on_laid_out(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {
                self.laid_out.insert(*ctx.render_object_id);

                self.inner.on_laid_out(ctx, render_object);
            }
        }

        let mut changed = FxHashMap::default();

        let mut laid_out = FxHashSet::default();

        // TODO: can we have this collect only the relayout boundaries?
        let mut needs_layout = needs_layout.into_iter().collect::<Vec<_>>();

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

                            changed: &mut changed,

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

        for (render_object_id, (new_size, new_offset)) in changed {
            if let Some(render_view) = self.render_views.get_mut(render_object_id) {
                let (view_object_id, view) = match render_view {
                    RenderView::Owner(ref mut view) => (render_object_id, view),
                    RenderView::Within(view_object_id) => {
                        let view_object_id = *view_object_id;

                        match self.render_views.get_mut(view_object_id) {
                            Some(RenderView::Owner(view)) => (view_object_id, view),
                            _ => panic!(
                                "render object supplied an incorrect render view while syncing"
                            ),
                        }
                    }
                };

                if let Some(new_size) = new_size {
                    view.on_size_changed(render_object_id, new_size);
                }

                if let Some(new_offset) = new_offset {
                    view.on_offset_changed(render_object_id, new_offset);
                }

                self.needs_sync.insert(view_object_id, ());
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn paint(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!(?render_object_id, "painting render object");

        let render_object = self
            .tree
            .get_mut(render_object_id)
            .expect("render object missing while flushing paint");

        let Some(render_view) = self.render_views.get_mut(render_object_id) else {
            return;
        };

        let canvas = render_object.paint();

        let (view_object_id, view) = match render_view {
            RenderView::Owner(ref mut view) => (render_object_id, view),
            RenderView::Within(view_object_id) => {
                let view_object_id = *view_object_id;

                match self.render_views.get_mut(view_object_id) {
                    Some(RenderView::Owner(view)) => (view_object_id, view),
                    _ => panic!(
                        "render object supplied an incorrect render view while flushing paint"
                    ),
                }
            }
        };

        view.on_paint(render_object_id, canvas);

        self.needs_sync.insert(view_object_id, ());
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn sync_views(&mut self) {
        for render_object_id in self.needs_sync.drain().map(|(id, _)| id) {
            tracing::trace!(?render_object_id, "syncing render object's view");

            if let Some(render_view) = self.render_views.get_mut(render_object_id) {
                let view = match render_view {
                    RenderView::Owner(ref mut view) => view,
                    RenderView::Within(view_object_id) => {
                        let view_object_id = *view_object_id;

                        match self.render_views.get_mut(view_object_id) {
                            Some(RenderView::Owner(view)) => view,
                            _ => panic!(
                                "render object supplied an incorrect render view while syncing"
                            ),
                        }
                    }
                };

                view.on_sync();
            }
        }
    }
}

impl AsRef<Tree<RenderObjectId, RenderObject>> for RenderingTree {
    fn as_ref(&self) -> &Tree<RenderObjectId, RenderObject> {
        &self.tree
    }
}
