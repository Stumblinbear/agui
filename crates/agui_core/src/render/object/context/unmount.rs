use crate::{
    element::ContextRenderObject,
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    render::RenderObjectId,
};

pub struct RenderObjectUnmountContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub render_object_id: &'ctx RenderObjectId,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectUnmountContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectUnmountContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextRenderObject for RenderObjectUnmountContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}
