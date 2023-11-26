use std::ops::{Deref, DerefMut};

use crate::{
    element::{ContextRenderObject, ContextRenderObjects},
    plugin::{context::ContextPlugins, Plugins},
    render::{RenderObject, RenderObjectId},
    unit::{HitTestResult, Size},
    util::tree::Tree,
};

mod iter;

pub use iter::*;

pub struct RenderObjectHitTestContext<'ctx> {
    pub plugins: &'ctx Plugins,

    pub render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,

    pub size: &'ctx Size,

    pub children: &'ctx [RenderObjectId],

    pub(crate) result: &'ctx mut HitTestResult,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectHitTestContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl ContextRenderObjects for RenderObjectHitTestContext<'_> {
    fn render_objects(&self) -> &Tree<RenderObjectId, RenderObject> {
        self.render_object_tree
    }
}

impl ContextRenderObject for RenderObjectHitTestContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl RenderObjectHitTestContext<'_> {
    pub fn size(&self) -> Size {
        *self.size
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn iter_children(&mut self) -> IterChildrenHitTest {
        IterChildrenHitTest {
            front_index: 0,
            back_index: self.children.len(),

            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            children: self.children,

            result: self.result,
        }
    }
}

impl Deref for RenderObjectHitTestContext<'_> {
    type Target = HitTestResult;

    fn deref(&self) -> &Self::Target {
        self.result
    }
}

impl DerefMut for RenderObjectHitTestContext<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.result
    }
}
