use crate::{
    element::ContextRenderObject,
    render::{
        object::{context::IterChildrenLayout, RenderObject},
        RenderObjectId,
    },
    util::tree::Tree,
};

pub struct RenderObjectIntrinsicSizeContext<'ctx> {
    pub(crate) render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub render_object_id: &'ctx RenderObjectId,

    pub children: &'ctx [RenderObjectId],
}

impl ContextRenderObject for RenderObjectIntrinsicSizeContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl<'ctx> RenderObjectIntrinsicSizeContext<'ctx> {
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
}
