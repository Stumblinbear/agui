use core::panic;
use std::{collections::VecDeque, hash::BuildHasherDefault, rc::Rc};

use rustc_hash::{FxHashSet, FxHasher};
use slotmap::SparseSecondaryMap;

use crate::{
    element::{
        Element, ElementId, ElementType, RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    engine::Dirty,
    render::{
        binding::RenderView,
        object::{layout_data::LayoutDataUpdate, RenderObject, RenderObjectLayoutContext},
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

    layout_changed:
        SparseSecondaryMap<RenderObjectId, LayoutDataUpdate, BuildHasherDefault<FxHasher>>,
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
        // let mut relayout_queue = self
        //     .needs_layout
        //     .drain()
        //     .filter(|render_object_id| self.render_object_tree.contains(*render_object_id))
        //     .collect::<Vec<_>>();

        // relayout_queue.sort_by_cached_key(|render_object_id| {
        //     self.render_object_tree
        //         .get_depth(*render_object_id)
        //         .unwrap()
        // });

        for render_object_id in self.needs_layout.drain() {
            tracing::trace!(?render_object_id, "laying out render object");

            let Some(render_node) = self.tree.get_node(render_object_id) else {
                tracing::warn!(
                    ?render_object_id,
                    "layout queued for a render object that does not exist"
                );

                continue;
            };

            let (render_object, children) = render_node.into();

            if !render_object.is_relayout_boundary(Constraints::default()) {
                tracing::warn!(
                    ?render_object_id,
                    ?render_object,
                    "layout queued for a render object that is not a relayout boundary"
                );

                return;
            }

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

                    layout_changed: &mut self.layout_changed,
                },
                Constraints::default(),
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

            if let Some(render_view) = render_object.render_view() {
                if let Some(size) = layout_update.size {
                    render_view.on_size_changed(render_object_id, size);
                }

                if let Some(offset) = layout_update.offset {
                    render_view.on_offset_changed(render_object_id, offset);
                }
            }

            render_object.apply_layout_data(layout_update);
        }
    }

    #[tracing::instrument(level = "trace", skip(self))]
    pub fn flush_needs_paint(&mut self) {
        for render_object_id in self.needs_paint.drain() {
            let render_object = self
                .tree
                .get_mut(render_object_id)
                .expect("render object missing while flushing paint");

            if let Some(render_view) = render_object.render_view() {
                render_view.on_paint(render_object_id, render_object.paint());
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

                if let ElementType::View(ref element) = parent_element.as_ref() {
                    return (
                        Some(parent_render_object_id),
                        Some(parent_render_object_id),
                        Some(RenderView::new(
                            parent_render_object_id,
                            Rc::clone(element.binding()),
                        )),
                    );
                }

                let parent_render_object = self
                    .tree
                    .get(parent_render_object_id)
                    .expect("parent render object missing while creating render object");

                (
                    parent_render_object.relayout_boundary_id(),
                    Some(parent_render_object_id),
                    parent_render_object.render_view().cloned(),
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
            .add_with_key(parent_render_object_id, |render_object_id| {
                let mut render_object =
                    element.create_render_object(&mut RenderObjectCreateContext {
                        element_id: &element_id,
                    });

                element.set_render_object_id(render_object_id);

                if let ElementType::View(element) = element.as_mut() {
                    let view_binding = Rc::clone(element.binding());

                    // Attach the render object as the root of its own view
                    view_binding.on_attach(None, render_object_id);

                    render_object
                        .set_render_view(Some(RenderView::new(render_object_id, view_binding)));
                } else if let Some(parent_render_view) = parent_render_view {
                    parent_render_view.on_attach(parent_render_object_id, render_object_id);

                    render_object.set_render_view(Some(parent_render_view));
                };

                if let Some(relayout_boundary_id) = relayout_boundary_id {
                    render_object.apply_layout_data(LayoutDataUpdate {
                        relayout_boundary_id: Some(Some(relayout_boundary_id)),
                        ..Default::default()
                    });

                    self.needs_layout.insert(relayout_boundary_id);
                }

                self.needs_paint.insert(render_object_id);

                render_object
            });
    }
}
