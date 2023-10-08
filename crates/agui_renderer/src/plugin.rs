use agui_core::{
    element::{ContextElement, ElementId},
    plugin::{
        context::{PluginElementMountContext, PluginElementUnmountContext},
        Plugin,
    },
};

use crate::{manager::RenderViewManager, RenderViewId};

#[derive(Default)]
pub struct RenderViewPlugin {
    manager: RenderViewManager,
}

impl Plugin for RenderViewPlugin {
    fn on_element_mount(&mut self, ctx: PluginElementMountContext) {
        self.manager
            .add(ctx.get_parent_element_id(), ctx.get_element_id());
    }

    fn on_element_remount(&mut self, ctx: PluginElementMountContext) {
        let element_id = ctx.get_element_id();

        let parent_render_view_id = ctx
            .get_parent_element_id()
            .and_then(|element_id| self.manager.get_view(element_id));

        self.manager
            .update_render_view(ctx.get_elements(), element_id, parent_render_view_id);
    }

    fn on_element_unmount(&mut self, ctx: PluginElementUnmountContext) {
        self.manager.remove(ctx.get_element_id());
    }
}

impl RenderViewPlugin {
    pub(crate) fn create_render_view(&mut self, element_id: ElementId) {
        self.manager.create_render_view(element_id);
    }

    pub fn get_boundary(&self, render_view_id: RenderViewId) -> Option<ElementId> {
        self.manager.get_boundary(render_view_id)
    }

    pub fn get_view(&self, element_id: ElementId) -> Option<RenderViewId> {
        self.manager.get_view(element_id)
    }
}
