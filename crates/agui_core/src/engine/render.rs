use core::panic;
use std::{collections::VecDeque, hash::BuildHasherDefault};

use rustc_hash::{FxHashSet, FxHasher};
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    element::{
        Element, ElementId, ElementType, RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    engine::Dirty,
    render::{
        object::{layout_data::LayoutDataUpdate, RenderObject, RenderObjectLayoutContext},
        view::RenderView,
        RenderObjectId,
    },
    unit::Constraints,
    util::tree::Tree,
};

#[derive(Default)]
pub struct RenderManager {
    tree: Tree<RenderObjectId, RenderObject>,

    create_render_object: VecDeque<ElementId>,
    update_render_object: FxHashSet<ElementId>,
    sync_render_object_children: FxHashSet<ElementId>,
    forgotten_elements: FxHashSet<ElementId>,

    needs_layout: Dirty<RenderObjectId>,
    needs_paint: Dirty<RenderObjectId>,

    cached_constraints: SecondaryMap<RenderObjectId, Constraints>,

    layout_changed:
        SparseSecondaryMap<RenderObjectId, LayoutDataUpdate, BuildHasherDefault<FxHasher>>,

    needs_sync: SparseSecondaryMap<RenderObjectId, (), BuildHasherDefault<FxHasher>>,
}

impl RenderManager {
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

    pub fn on_children_changed(&mut self, element_id: ElementId) {
        self.sync_render_object_children.insert(element_id);
    }

    pub fn forget_element(&mut self, element_id: ElementId) {
        self.forgotten_elements.insert(element_id);
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_layout(&mut self) {
        let mut relayout_queue = self
            .needs_layout
            .drain()
            .filter(|render_object_id| self.tree.contains(*render_object_id))
            .collect::<Vec<_>>();

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

            let (render_object, children) = render_node.into();

            let constraints = self
                .tree
                .get_parent(render_object_id)
                .and_then(|parent_id| self.cached_constraints.get(parent_id).copied())
                .unwrap_or_default();

            tracing::trace!(?render_object_id, ?constraints, "layout constraints");

            // TODO: we need a way to allow render objects to create new elements
            // during layout. This is important for performant responsivity, since
            // some elements may want to change which children they have based on
            // the parent element's size constraints.
            render_object.layout(
                &mut RenderObjectLayoutContext {
                    render_object_tree: &self.tree,

                    parent_uses_size: &false,

                    relayout_boundary_id: &Some(render_object_id),

                    render_object_id: &render_object_id,

                    children,

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

            if let Some(render_view) = render_object.render_view_mut() {
                let (render_view_id, view) = match render_view {
                    RenderView::Owner(ref mut view) => (render_object_id, view),
                    RenderView::Within(render_view_id) => {
                        let render_view_id = *render_view_id;

                        let render_view = self
                            .tree
                            .get_mut(render_view_id)
                            .expect("render view missing while applying layout");

                        match render_view.render_view_mut() {
                            Some(RenderView::Owner(view) ) => (render_view_id, view),
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

                if layout_update.size.is_some() || layout_update.offset.is_some() {
                    self.needs_sync.insert(render_view_id, ());
                }
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_needs_paint(&mut self) {
        for render_object_id in self.needs_paint.drain() {
            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while flushing paint");

            let canvas = if render_object.render_view().is_some() {
                render_object.paint()
            } else {
                continue;
            };

            let render_view = render_object.render_view_mut().unwrap();

            let (render_view_id, view) = match render_view {
                RenderView::Owner(ref mut view) => (render_object_id, view),
                RenderView::Within(render_view_id) => {
                    let render_view_id = *render_view_id;

                    let render_view = self
                        .tree
                        .get_mut(render_view_id)
                        .expect("render view missing while flushing paint");

                    match render_view.render_view_mut() {
                        Some(RenderView::Owner(view)) => (render_view_id, view),
                        _ => panic!(
                            "render object supplied an incorrect render view while flushing paint"
                        ),
                    }
                }
            };

            view.on_paint(render_object_id, canvas);

            self.needs_sync.insert(render_view_id, ());
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_view_sync(&mut self) {
        for render_object_id in self.needs_sync.drain().map(|(id, _)| id) {
            // self.tree
            //     .get_mut(render_object_id)
            //     .expect("render object missing while syncing")
            //     .render_view()
            //     .expect("render object has no view while syncing")
            //     .on_sync()

            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while syncing");

            if let Some(render_view) = render_object.render_view_mut() {
                let view = match render_view {
                    RenderView::Owner(ref mut view) => view,
                    RenderView::Within(render_view_id) => {
                        let render_view_id = *render_view_id;

                        let render_view = self
                            .tree
                            .get_mut(render_view_id)
                            .expect("render view missing while syncing");

                        match render_view.render_view_mut() {
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

    #[tracing::instrument(level = "trace", skip(self, element_tree))]
    pub fn sync_render_objects(&mut self, element_tree: &mut Tree<ElementId, Element>) {
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
            .sync_render_object_children
            .drain()
            .filter(|element_id| !self.forgotten_elements.contains(element_id))
        {
            // Elements that were removed should still be available in the tree, so this should
            // never fail.
            let (element, children) = element_tree
                .get_node(element_id)
                .expect("element missing while syncing render object children")
                .into();

            if let Some(render_object_id) = element.render_object_id() {
                let mut first_child_render_object_id = None;

                let children = children.to_vec();

                // Yank the render objects of the element's children from wheverever they are in
                // the tree to the end of the list.
                for child_id in children {
                    let child_render_object_id = element_tree
                        .get(child_id)
                        .expect("child element missing while syncing render object children")
                        .render_object_id()
                        .expect("child element has no render object");

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
        for element_id in self.forgotten_elements.iter().copied() {
            if let Some(render_object_id) = element_tree
                .get(element_id)
                .and_then(|element| element.render_object_id())
            {
                self.tree.remove(render_object_id, false);

                self.cached_constraints.remove(render_object_id);
            }
        }

        for element_id in self
            .update_render_object
            .drain()
            .filter(|element_id| !self.forgotten_elements.contains(element_id))
        {
            let element = element_tree
                .get(element_id)
                .expect("element missing while updating render objects");

            let render_object_id = element
                .render_object_id()
                .expect("element has no render object to update");

            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while updating");

            let element = element_tree
                .get_mut(element_id)
                .expect("element missing while creating render objects");

            let mut needs_layout = false;
            let mut needs_paint = false;

            element.update_render_object(
                &mut RenderObjectUpdateContext {
                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,

                    element_id: &element_id,

                    render_object_id: &render_object_id,
                },
                render_object,
            );

            if needs_layout {
                self.needs_layout.insert(
                    render_object
                        .relayout_boundary_id()
                        .unwrap_or(render_object_id),
                )
            }

            if needs_paint {
                self.needs_paint.insert(render_object_id);
            }
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn create_render_object(
        &mut self,
        element_tree: &mut Tree<ElementId, Element>,
        element_id: ElementId,
    ) {
        let (relayout_boundary_id, parent_render_object_id, parent_render_view) = element_tree
            .get_parent(element_id)
            .map(|parent_element_id| {
                let parent_element = element_tree
                    .get(parent_element_id)
                    .expect("parent element missing while creating render object");

                let parent_render_object_id = parent_element
                    .render_object_id()
                    .expect("parent element has no render object while creating render object");

                if let ElementType::View(_) = parent_element.as_ref() {
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
                        Some(RenderView::Within(render_view_id)) => Some(*render_view_id),
                        None => None,
                    },
                )
            })
            .unwrap_or_default();

        let element = element_tree
            .get_mut(element_id)
            .expect("element missing while creating render objects");

        if let Some(render_object_id) = element.render_object_id() {
            panic!(
                "element already has a render object: {:?}",
                render_object_id
            );
        }

        self.tree
            .add_with_key(parent_render_object_id, |tree, render_object_id| {
                element.set_render_object_id(render_object_id);

                let mut render_object =
                    element.create_render_object(&mut RenderObjectCreateContext {
                        element_id: &element_id,
                    });

                let relayout_boundary_id = relayout_boundary_id.unwrap_or(render_object_id);

                render_object.apply_layout_data(&LayoutDataUpdate {
                    relayout_boundary_id: Some(Some(relayout_boundary_id)),
                    ..Default::default()
                });

                if let ElementType::View(element) = element.as_mut() {
                    let mut view = element.create_view();

                    // Attach the render object as the root of its own view
                    view.on_attach(None, render_object_id);

                    render_object.set_render_view(Some(RenderView::Owner(view)));
                } else if let Some(parent_render_view) = parent_render_view {
                    let view = match tree
                        .get_mut(parent_render_view)
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

                    render_object.set_render_view(Some(RenderView::Within(parent_render_view)));
                };

                self.needs_layout.insert(relayout_boundary_id);

                self.needs_paint.insert(render_object_id);

                render_object
            });
    }
}