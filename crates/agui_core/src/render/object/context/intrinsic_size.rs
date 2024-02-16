use crate::{
    element::ContextRenderObject,
    engine::rendering::{strategies::RenderingTreeTextLayoutStrategy, RenderingTree},
    render::{object::context::IterChildrenLayout, RenderObjectId},
};

pub struct RenderObjectIntrinsicSizeContext<'ctx> {
    pub(crate) tree: &'ctx RenderingTree,

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
            tree: self.tree,

            index: 0,

            children: self.children,
        }
    }

    pub fn text_layout(&self) -> Option<&dyn RenderingTreeTextLayoutStrategy> {
        Some(self.tree.get_view(*self.render_object_id)?.text_layout())
    }
}
