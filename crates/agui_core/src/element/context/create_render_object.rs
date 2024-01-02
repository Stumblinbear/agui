use crate::{
    element::ElementId,
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
};

use super::ContextElement;

pub struct RenderObjectCreateContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_id: &'ctx ElementId,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectCreateContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectCreateContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextElement for RenderObjectCreateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}
