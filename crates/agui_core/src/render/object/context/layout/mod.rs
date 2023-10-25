use crate::{
    plugin::{context::ContextPlugins, Plugins},
    render::{RenderObject, RenderObjectId},
    unit::Offset,
    util::tree::Tree,
};

mod iter;

pub use iter::*;

use super::{ContextRenderObject, ContextRenderObjects};

pub struct RenderObjectLayoutContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub(crate) render_object_tree: &'ctx mut Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,

    pub children: &'ctx [RenderObjectId],
    pub offsets: &'ctx mut [Offset],
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectLayoutContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl ContextRenderObjects for RenderObjectLayoutContext<'_> {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        self.render_object_tree
    }
}

impl ContextRenderObject for RenderObjectLayoutContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl RenderObjectLayoutContext<'_> {
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&self) -> IterChildrenLayout {
        IterChildrenLayout {
            index: 0,

            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            children: self.children,
        }
    }

    pub fn iter_children_mut(&mut self) -> IterChildrenLayoutMut {
        IterChildrenLayoutMut {
            index: 0,

            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            children: self.children,
            offsets: self.offsets,
        }
    }
}
