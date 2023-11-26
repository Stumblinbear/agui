use rustc_hash::FxHashMap;

use agui_core::{
    element::{Element, ElementId},
    plugin::context::{ContextPlugins, ContextPluginsMut},
    util::tree::Tree,
};
use slotmap::SlotMap;

use crate::plugin::RenderViewPlugin;

use super::RenderViewId;

#[derive(Default)]
pub struct RenderViewManager {
    map: FxHashMap<ElementId, RenderViewId>,

    boundaries: SlotMap<RenderViewId, ElementId>,
}

impl RenderViewManager {
    pub fn of<'ctx>(ctx: &impl ContextPlugins<'ctx>) -> Option<&Self> {
        ctx.plugins()
            .get::<RenderViewPlugin>()
            .map(|plugin| &plugin.manager)
    }

    pub fn of_mut<'ctx>(ctx: &mut impl ContextPluginsMut<'ctx>) -> Option<&mut Self> {
        ctx.plugins_mut()
            .get_mut::<RenderViewPlugin>()
            .map(|plugin| &mut plugin.manager)
    }

    pub fn get_view(&self, element_id: ElementId) -> Option<RenderViewId> {
        self.map.get(&element_id).copied()
    }

    pub fn get_boundary(&self, render_view_id: RenderViewId) -> Option<ElementId> {
        self.boundaries.get(render_view_id).copied()
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn create_render_view(&mut self, element_id: ElementId) -> RenderViewId {
        assert!(
            !self.map.contains_key(&element_id),
            "element already exists in the render view manager"
        );

        let render_view_id = self.boundaries.insert(element_id);

        self.map.insert(element_id, render_view_id);

        render_view_id
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn add(&mut self, parent_element_id: Option<ElementId>, element_id: ElementId) {
        if self.map.contains_key(&element_id) {
            return;
        }

        tracing::trace!("attaching render view");

        let parent_render_view_id = parent_element_id
            .map(|parent_element_id| {
                self.get_view(parent_element_id).expect(
                    "cannot add element to the render view manager as the parent does not exist",
                )
            })
            .unwrap_or_default();

        self.map.insert(element_id, parent_render_view_id);
    }

    #[tracing::instrument(skip(self, element_tree))]
    pub(crate) fn update_render_view(
        &mut self,
        element_tree: &Tree<ElementId, Element>,
        element_id: ElementId,
        new_render_view_id: Option<RenderViewId>,
    ) {
        let current_render_view_id = self.map.get(&element_id).copied();

        if new_render_view_id == current_render_view_id {
            return;
        }

        // If this element is the creator of a render view, then we don't need to do anything.
        if let Some(current_render_view_id) = current_render_view_id {
            if self.get_boundary(current_render_view_id) == Some(element_id) {
                return;
            }
        }

        if let Some(new_render_view_id) = new_render_view_id {
            self.map.insert(element_id, new_render_view_id);
        } else {
            // Remove this element from the render view.
            self.map.remove(&element_id);
        }

        for child_id in element_tree
            .get_children(element_id)
            .cloned()
            .unwrap_or_default()
        {
            self.update_render_view(element_tree, child_id, new_render_view_id);
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn remove(&mut self, element_id: ElementId) {
        if let Some(render_view_id) = self.map.remove(&element_id) {
            // If this element is the one that created the render view, remove it from the map.
            if self.get_boundary(render_view_id) == Some(element_id) {
                self.boundaries.remove(render_view_id);
            }
        }
    }
}
