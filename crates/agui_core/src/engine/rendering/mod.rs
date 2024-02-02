use core::panic;
use std::{collections::VecDeque, hash::BuildHasherDefault};

use rustc_hash::{FxHashSet, FxHasher};
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    element::{Element, ElementId, RenderObjectCreateContext, RenderObjectUpdateContext},
    engine::{
        elements::ElementTree,
        rendering::scheduler::{RenderingScheduler, RenderingSchedulerStrategy},
        Dirty,
    },
    render::{
        object::{layout_data::LayoutDataUpdate, RenderObject, RenderObjectLayoutContext},
        view::RenderView,
        RenderObjectId,
    },
    unit::Constraints,
    util::tree::Tree,
};

mod builder;
pub mod scheduler;

pub use builder::*;

pub struct RenderManager<SB = ()> {
    scheduler: SB,

    tree: Tree<RenderObjectId, RenderObject>,

    elements: SecondaryMap<ElementId, RenderObjectId>,

    create_render_object: VecDeque<ElementId>,
    update_render_object: FxHashSet<ElementId>,
    forgotten_elements: FxHashSet<ElementId>,

    dirty_deferred_elements: FxHashSet<ElementId>,
    dirty_layout_boundaries: FxHashSet<RenderObjectId>,

    needs_layout: Dirty<RenderObjectId>,
    needs_paint: Dirty<RenderObjectId>,

    cached_constraints: SecondaryMap<RenderObjectId, Constraints>,

    layout_changed:
        SparseSecondaryMap<RenderObjectId, LayoutDataUpdate, BuildHasherDefault<FxHasher>>,

    needs_sync: SparseSecondaryMap<RenderObjectId, (), BuildHasherDefault<FxHasher>>,
}

impl RenderManager<()> {
    pub fn builder() -> RenderManagerBuilder<()> {
        RenderManagerBuilder::default()
    }
}

impl<SB> RenderManager<SB> {
    pub fn tree(&self) -> &Tree<RenderObjectId, RenderObject> {
        &self.tree
    }

    pub fn on_create_element(&mut self, element_id: ElementId) {
        self.create_render_object.push_back(element_id);

        // It's possible for the widget to be created then rebuilt in the same frame,
        // we want to make absolutely sure that this case is handled properly so we
        // cause this case to happen randomly in debug builds.
        #[cfg(debug_assertions)]
        if rand::random::<bool>() {
            self.update_render_object.insert(element_id);
        }
    }

    pub fn on_needs_update(&mut self, element_id: ElementId) {
        self.update_render_object.insert(element_id);
    }

    pub fn on_needs_layout(&mut self, render_object_id: RenderObjectId) {
        self.needs_layout.insert(render_object_id);
    }

    pub fn on_needs_paint(&mut self, render_object_id: RenderObjectId) {
        self.needs_paint.insert(render_object_id);
    }

    pub fn forget_element(&mut self, element_id: ElementId) {
        self.forgotten_elements.insert(element_id);
    }

    pub fn does_need_sync(&self) -> bool {
        !self.create_render_object.is_empty()
            || !self.update_render_object.is_empty()
            || !self.forgotten_elements.is_empty()
    }

    pub fn does_need_update(&self) -> bool {
        !self.needs_layout.is_empty() || !self.needs_paint.is_empty() || !self.needs_sync.is_empty()
    }

    pub fn flush_needs_layout(&mut self) {
        self.needs_layout.process(|render_object_id| {
            if let Some(render_object) = self.tree.get(render_object_id) {
                self.dirty_layout_boundaries.insert(
                    render_object
                        .relayout_boundary_id()
                        .unwrap_or(render_object_id),
                );
            }
        });
    }

    // #[tracing::instrument(level = "trace", skip_all)]
    // pub fn resolve_deferred_element(&mut self, sync_data: SyncTreeData) -> Option<ElementId> {
    //     if self.dirty_deferred_elements.is_empty() {
    //         return None;
    //     }

    //     // In order to resolve deferred elements, we need to perform layout on them.

    //     // TODO: only perform layout on the layout boundaries that contain deferred
    //     // elements.

    //     for element_id in self.dirty_deferred_elements.drain() {
    //         let render_object_id =
    //             self.elements.get(element_id).copied().expect(
    //                 "deferred element has no render object while resolving deferred elements",
    //             );

    //         let render_object = self
    //             .tree
    //             .get_mut(render_object_id)
    //             .expect("render object missing while resolving deferred elements");

    //         let Some(relayout_boundary_id) = render_object.relayout_boundary_id() else {
    //             continue;
    //         };

    //         println!("relayout boundary: {:?}", relayout_boundary_id);
    //         println!(
    //             "dirty layout boundaries: {:?}",
    //             self.dirty_layout_boundaries,
    //         );

    //         let deferred_resolver = sync_data
    //             .deferred_resolvers
    //             .get_mut(element_id)
    //             .expect("deferred resolver missing while resolving deferred elements");

    //         let constraints = if !self.dirty_layout_boundaries.contains(&relayout_boundary_id) {
    //             // If the deferred element is not going to be laid out, we use the cached
    //             // constraints to resolve it.
    //             self.cached_constraints
    //                 .get(render_object_id)
    //                 .copied()
    //                 .expect("deferred element was not laid out")
    //         } else {
    //             // TODO: perform layout on the boundary
    //             Constraints::expand()
    //         };

    //         if !deferred_resolver.resolve(constraints) {
    //             // The deferred element did not change, so we can skip it.
    //             continue;
    //         }

    //         // TODO: call back to the widget tree to build the element
    //     }

    //     None
    // }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_layout(&mut self) -> bool {
        if self.dirty_layout_boundaries.is_empty() {
            return false;
        }

        let mut relayout_queue = self.dirty_layout_boundaries.drain().collect::<Vec<_>>();

        relayout_queue
            .sort_by_cached_key(|render_object_id| self.tree.get_depth(*render_object_id).unwrap());

        for render_object_id in relayout_queue {
            tracing::trace!(?render_object_id, "laying out render object");

            // It's likely that a nested render object will have already been processed by
            // a previous iteration of the loop, so we can skip it here.
            if self.layout_changed.contains_key(render_object_id) {
                tracing::trace!(
                    ?render_object_id,
                    "render object has already been laid out, skipping"
                );

                continue;
            }

            let Some(render_node) = self.tree.get_node(render_object_id) else {
                tracing::warn!(
                    ?render_object_id,
                    "layout queued for a render object that does not exist"
                );

                continue;
            };

            let constraints = self
                .tree
                .get_parent(render_object_id)
                .and_then(|parent_id| self.cached_constraints.get(*parent_id).copied())
                .unwrap_or_default();

            tracing::trace!(?render_object_id, ?constraints, "layout constraints");

            // TODO: we need a way to allow render objects to create new elements
            // during layout. This is important for performant responsivity, since
            // some elements may want to change which children they have based on
            // the parent element's size constraints.
            render_node.borrow().layout(
                &mut RenderObjectLayoutContext {
                    render_object_tree: &self.tree,

                    parent_uses_size: &false,

                    relayout_boundary_id: &Some(render_object_id),

                    render_object_id: &render_object_id,

                    children: render_node.children(),

                    constraints: &mut self.cached_constraints,

                    layout_changed: &mut self.layout_changed,
                },
                constraints,
            );
        }

        for (render_object_id, layout_update) in self.layout_changed.drain() {
            tracing::trace!(
                ?render_object_id,
                changed = ?layout_update,
                "applying layout changes to render object"
            );

            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("cannot apply layout to a non-existent render object");

            render_object.apply_layout_data(&layout_update);

            let does_paint = render_object.does_paint();

            if let Some(view_object) = render_object.render_view_mut() {
                let (view_object_id, view) = match view_object {
                    RenderView::Owner(ref mut view) => (render_object_id, view),
                    RenderView::Within(view_object_id) => {
                        let view_object_id = *view_object_id;

                        let view_object = self
                            .tree
                            .get_mut(view_object_id)
                            .expect("render view missing while applying layout");

                        match view_object.render_view_mut() {
                            Some(RenderView::Owner(view) ) => (view_object_id, view),
                            _ => panic!("render object supplied an incorrect render view while applying layout"),
                        }
                    }
                };

                if let Some(size) = layout_update.size {
                    view.on_size_changed(render_object_id, size);
                }

                if let Some(offset) = layout_update.offset {
                    view.on_offset_changed(render_object_id, offset);
                }

                if layout_update.size.is_some() && does_paint {
                    // TODO: how can we reduce repaints for render objects that don't
                    // need to be repainted when their size changes?
                    self.needs_paint.insert(render_object_id);
                } else if layout_update.offset.is_some() {
                    self.needs_sync.insert(view_object_id, ());
                }
            }
        }

        true
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_paint(&mut self) -> bool {
        self.needs_paint.process(|render_object_id| {
            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while flushing paint");

            let canvas = if render_object.render_view().is_some() {
                render_object.paint()
            } else {
                return;
            };

            let view_object = render_object.render_view_mut().unwrap();

            let (view_object_id, view) = match view_object {
                RenderView::Owner(ref mut view) => (render_object_id, view),
                RenderView::Within(view_object_id) => {
                    let view_object_id = *view_object_id;

                    let view_object = self
                        .tree
                        .get_mut(view_object_id)
                        .expect("render view missing while flushing paint");

                    match view_object.render_view_mut() {
                        Some(RenderView::Owner(view)) => (view_object_id, view),
                        _ => panic!(
                            "render object supplied an incorrect render view while flushing paint"
                        ),
                    }
                }
            };

            view.on_paint(render_object_id, canvas);

            self.needs_sync.insert(view_object_id, ());
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn sync_views(&mut self) -> bool {
        if self.needs_sync.is_empty() {
            return false;
        }

        for render_object_id in self.needs_sync.drain().map(|(id, _)| id) {
            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while syncing");

            if let Some(view_object) = render_object.render_view_mut() {
                let view = match view_object {
                    RenderView::Owner(ref mut view) => view,
                    RenderView::Within(view_object_id) => {
                        let view_object_id = *view_object_id;

                        let view_object = self
                            .tree
                            .get_mut(view_object_id)
                            .expect("render view missing while syncing");

                        match view_object.render_view_mut() {
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

        true
    }
}

impl<SB> RenderManager<SB>
where
    SB: RenderingSchedulerStrategy,
{
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn sync_render_objects(&mut self, element_tree: &ElementTree) {
        // No need to update render objects that are about to be created.
        if self.update_render_object.len() <= self.create_render_object.len() {
            self.update_render_object
                .retain(|element_id| !self.create_render_object.contains(element_id));
        } else {
            self.create_render_object.iter().for_each(|element_id| {
                self.update_render_object.remove(element_id);
            });
        }

        while let Some(element_id) = self.create_render_object.pop_front() {
            // No need to create render objects whose elements are going to be removed.
            if self.forgotten_elements.contains(&element_id) {
                continue;
            }

            self.create_render_object(element_tree, element_id);
        }

        for element_id in self
            .update_render_object
            .drain()
            .filter(|element_id| !self.forgotten_elements.contains(element_id))
        {
            let element_node = element_tree
                .as_ref()
                .get_node(element_id)
                .expect("element missing while yodatubg render objects");

            let render_object_id = self
                .elements
                .get(element_id)
                .copied()
                .expect("element has no render object to update");

            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while updating");

            let element = element_node.borrow();

            if matches!(element, Element::Deferred(_)) {
                self.dirty_deferred_elements.insert(element_id);
            }

            element.update_render_object(
                &mut RenderObjectUpdateContext {
                    scheduler: &mut RenderingScheduler::new(&render_object_id)
                        .with_strategy(&mut self.scheduler),

                    needs_layout: &mut self.needs_layout,
                    needs_paint: &mut self.needs_paint,

                    element_id: &element_id,

                    render_object_id: &render_object_id,
                },
                render_object,
            );

            if let Some(render_object_id) = self.elements.get(element_id).copied() {
                let mut first_child_render_object_id = None;

                // Reorder the element children's render objects to match the element's children.
                for child_element_id in element_node.children().iter().copied() {
                    let child_render_object_id =
                        self.elements.get(child_element_id).copied().expect(
                            "child element has no render object while syncing render object",
                        );

                    self.tree
                        .reparent(Some(render_object_id), child_render_object_id);

                    if first_child_render_object_id.is_none() {
                        first_child_render_object_id = Some(child_render_object_id);
                    }
                }

                let mut found_child = false;

                // Remove any render objects that were previously children but are no longer.
                // Since the `reparent` call reorders them to the end of the list, we can remove
                // every child from the beginning of the list until we reach the first child
                // that is still a child of the element.
                self.tree.retain_children(render_object_id, |child_id| {
                    if first_child_render_object_id == Some(*child_id) {
                        found_child = true;
                    }

                    found_child
                });
            }
        }

        // Remove any render objects owned by elements that are being removed.
        for element_id in self.forgotten_elements.drain() {
            if let Some(render_object_id) = self.elements.remove(element_id) {
                if let Some(render_object) = self.tree.remove(render_object_id) {
                    if let Some(RenderView::Within(view_object_id)) = render_object.render_view() {
                        if let Some(view_object) = self.tree.get_mut(*view_object_id) {
                            match view_object.render_view_mut() {
                                Some(RenderView::Owner(view)) => view.on_detach(render_object_id),
                                _ => panic!(
                                    "render object supplied an incorrect render view while detatching"
                                ),
                            }
                        }
                    }
                }

                self.dirty_deferred_elements.remove(&element_id);

                self.cached_constraints.remove(render_object_id);
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self, element_tree))]
    pub fn create_render_object(&mut self, element_tree: &ElementTree, element_id: ElementId) {
        let (relayout_boundary_id, parent_render_object_id, parent_view_object) = element_tree.as_ref()
            .get_parent(element_id)
            .map(|parent_element_id| {
                let parent_element = element_tree.as_ref()
                    .get(*parent_element_id)
                    .expect("parent element missing while creating render object");

                let Some(parent_render_object_id) = self
                .elements
                .get(*parent_element_id)
                .copied() else {
                    panic!("parent element {:?} has no render object while creating render object {:?}", parent_element_id, element_id);
                };

                if let Element::View(_) = &parent_element {
                    return (
                        Some(parent_render_object_id),
                        Some(parent_render_object_id),
                        Some(parent_render_object_id),
                    );
                }

                let parent_render_object = self
                    .tree
                    .get(parent_render_object_id)
                    .expect("parent render object missing while creating render object");

                (
                    parent_render_object.relayout_boundary_id(),
                    Some(parent_render_object_id),
                    match parent_render_object.render_view() {
                        Some(RenderView::Owner(_)) => Some(parent_render_object_id),
                        Some(RenderView::Within(view_object_id)) => Some(*view_object_id),
                        None => None,
                    },
                )
            })
            .unwrap_or_default();

        if let Some(render_object_id) = self.elements.get(element_id) {
            panic!(
                "element already has a render object: {:?}",
                render_object_id
            );
        }

        let element = element_tree
            .as_ref()
            .get(element_id)
            .expect("element missing while creating render objects");

        self.tree
            .add_with_key(parent_render_object_id, |tree, render_object_id| {
                self.elements.insert(element_id, render_object_id);

                let mut render_object =
                    element.create_render_object(&mut RenderObjectCreateContext {
                        scheduler: &mut RenderingScheduler::new(&render_object_id)
                            .with_strategy(&mut self.scheduler),

                        element_id: &element_id,
                        render_object_id: &render_object_id,
                    });

                let relayout_boundary_id = relayout_boundary_id.unwrap_or(render_object_id);

                render_object.apply_layout_data(&LayoutDataUpdate {
                    relayout_boundary_id: Some(Some(relayout_boundary_id)),
                    ..Default::default()
                });

                if matches!(element, Element::Deferred(_)) {
                    self.dirty_deferred_elements.insert(element_id);
                } else if let Element::View(element) = &element {
                    let mut view = element.create_view();

                    // Attach the render object as the root of its own view
                    view.on_attach(None, render_object_id);

                    render_object.set_render_view(Some(RenderView::Owner(view)));
                } else if let Some(parent_view_object) = parent_view_object {
                    let view = match tree
                        .get_mut(parent_view_object)
                        .expect("parent render object missing while creating render objects")
                        .render_view_mut()
                        .expect("parent render object has no view while creating render objects")
                    {
                        RenderView::Owner(ref mut view) => view,
                        _ => panic!(
                            "parent render object is not a view owner while creating render objects"
                        ),
                    };

                    view.on_attach(parent_render_object_id, render_object_id);

                    render_object.set_render_view(Some(RenderView::Within(parent_view_object)));
                };

                self.needs_layout.insert(relayout_boundary_id);

                if render_object.does_paint() {
                    self.needs_paint.insert(render_object_id);
                }

                render_object
            });
    }
}
