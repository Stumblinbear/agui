use crate::{
    element::{ContextRenderObject, ContextRenderObjects},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    render::{RenderObject, RenderObjectId},
    util::tree::Tree,
};

mod hit_test;
mod intrinsic_size;
mod layout;
mod mount;
mod unmount;

pub use hit_test::*;
pub use intrinsic_size::*;
pub use layout::*;
pub use mount::*;
pub use unmount::*;

pub struct RenderObjectContext<'ctx> {
    pub plugins: &'ctx Plugins,

    pub render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl ContextRenderObjects for RenderObjectContext<'_> {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        self.render_object_tree
    }
}

impl ContextRenderObject for RenderObjectContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

pub struct RenderObjectContextMut<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub(crate) render_object_tree: &'ctx mut Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectContextMut<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectContextMut<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextRenderObjects for RenderObjectContextMut<'_> {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        self.render_object_tree
    }
}

impl ContextRenderObject for RenderObjectContextMut<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}
