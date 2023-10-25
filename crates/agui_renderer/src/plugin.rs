use agui_core::{
    element::ContextElement,
    plugin::{
        context::{
            PluginElementBuildContext, PluginElementMountContext, PluginElementRemountContext,
            PluginElementUnmountContext,
        },
        Plugin,
    },
};

use crate::manager::RenderViewManager;

#[derive(Default)]
pub(crate) struct RenderViewPlugin {
    pub manager: RenderViewManager,
}

impl Plugin for RenderViewPlugin {
    fn on_element_mount(&mut self, ctx: &mut PluginElementMountContext) {
        self.manager.add(ctx.parent_element_id(), ctx.element_id());
    }

    fn on_element_remount(&mut self, ctx: &mut PluginElementRemountContext) {
        let element_id = ctx.element_id();

        let parent_render_view_id = ctx
            .parent_element_id()
            .and_then(|element_id| self.manager.get_view(element_id));

        self.manager
            .update_render_view(ctx.elements(), element_id, parent_render_view_id);
    }

    fn on_element_unmount(&mut self, ctx: &mut PluginElementUnmountContext) {
        self.manager.remove(ctx.element_id());
    }

    fn on_element_build(&mut self, ctx: &mut PluginElementBuildContext) {}
}
