use crate::{
    element::{ContextRenderObject, ContextRenderObjects},
    render::{object::RenderObject, RenderObjectId},
    util::tree::Tree,
};

mod iter;
mod layout_result;

pub use iter::*;
pub use layout_result::*;
use rustc_hash::FxHashMap;

pub struct RenderObjectLayoutContext<'ctx> {
    pub render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,

    pub children: &'ctx [RenderObjectId],

    pub(crate) results: &'ctx mut FxHashMap<RenderObjectId, LayoutResult>,
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

            render_object_tree: self.render_object_tree,

            children: self.children,
        }
    }

    pub fn iter_children_mut(&mut self) -> IterChildrenLayoutMut {
        IterChildrenLayoutMut {
            index: 0,

            render_object_tree: self.render_object_tree,

            children: self.children,

            results: self.results,
        }
    }
}
