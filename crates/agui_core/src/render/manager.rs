use fnv::FnvHashMap;

use crate::{
    element::{Element, ElementId},
    util::tree::Tree,
};

use super::RenderContextId;

#[derive(Default)]
pub struct RenderContextManager {
    last_render_context_id: usize,

    map: FnvHashMap<ElementId, RenderContextId>,

    render_contexts: FnvHashMap<RenderContextId, ElementId>,
}

impl RenderContextManager {
    pub fn create_render_context(&mut self, element_id: ElementId) -> RenderContextId {
        assert!(
            !self.map.contains_key(&element_id),
            "element already exists in the render context manager"
        );

        self.last_render_context_id += 1;

        let render_context_id = RenderContextId::new(self.last_render_context_id);

        self.map.insert(element_id, render_context_id);
        self.render_contexts.insert(render_context_id, element_id);

        render_context_id
    }

    pub fn get_context(&self, element_id: ElementId) -> Option<RenderContextId> {
        self.map.get(&element_id).copied()
    }

    pub fn get_boundary(&self, render_context_id: RenderContextId) -> Option<ElementId> {
        self.render_contexts.get(&render_context_id).copied()
    }

    pub(crate) fn add(&mut self, parent_element_id: Option<ElementId>, element_id: ElementId) {
        tracing::trace!(
            element_id = &format!("{:?}", element_id),
            "attaching render context"
        );

        assert!(
            !self.map.contains_key(&element_id),
            "element already exists in the render context manager"
        );

        let parent_render_context_id = parent_element_id
            .map(|parent_element_id| {
                self.get_context(parent_element_id).expect(
                    "cannot add element to the render context manager as the parent does not exist",
                )
            })
            .unwrap_or_default();

        self.map.insert(element_id, parent_render_context_id);
    }

    pub(crate) fn update_render_context(
        &mut self,
        element_tree: &mut Tree<ElementId, Element>,
        element_id: ElementId,
        new_render_context_id: Option<RenderContextId>,
    ) {
        let current_render_context_id = self.map.get(&element_id).copied();

        if new_render_context_id == current_render_context_id {
            return;
        }

        // If this element is the creator of a render context, then we don't need to do anything.
        if let Some(current_render_context_id) = current_render_context_id {
            if self.render_contexts.get(&current_render_context_id) == Some(&element_id) {
                return;
            }
        }

        println!(
            "updating render context for {:?} from {:?} to {:?}",
            element_id, current_render_context_id, new_render_context_id
        );

        if let Some(new_render_context_id) = new_render_context_id {
            self.map.insert(element_id, new_render_context_id);
        } else {
            // Remove this element from the render context.
            self.map.remove(&element_id);
        }

        for child_id in element_tree
            .get_children(element_id)
            .cloned()
            .unwrap_or_default()
        {
            self.update_render_context(element_tree, child_id, new_render_context_id);
        }
    }

    pub(crate) fn remove(&mut self, element_id: ElementId) {
        if let Some(render_context_id) = self.map.remove(&element_id) {
            // If this element is the one that created the render context, remove it from the map.
            if self.render_contexts.get(&render_context_id) == Some(&element_id) {
                self.render_contexts.remove(&render_context_id);
            }
        }
    }
}
