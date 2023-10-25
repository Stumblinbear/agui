use crate::{
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    render::{RenderObject, RenderObjectId},
    util::tree::Tree,
};

use super::{ContextRenderObject, ContextRenderObjects};

pub struct RenderObjectMountContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub parent_render_object_id: Option<&'ctx RenderObjectId>,
    pub render_object_id: &'ctx RenderObjectId,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectMountContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectMountContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextRenderObjects for RenderObjectMountContext<'_> {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        self.render_object_tree
    }
}

impl ContextRenderObject for RenderObjectMountContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl RenderObjectMountContext<'_> {
    pub fn parent_render_object_id(&self) -> Option<RenderObjectId> {
        self.parent_render_object_id.copied()
    }
}
