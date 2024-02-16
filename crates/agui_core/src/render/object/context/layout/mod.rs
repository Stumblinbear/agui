use crate::{
    element::ContextRenderObject,
    engine::rendering::{
        strategies::{RenderingTreeLayoutStrategy, RenderingTreeTextLayoutStrategy},
        RenderingTree,
    },
    render::RenderObjectId,
};

mod iter;

pub use iter::*;

pub struct RenderObjectLayoutContext<'ctx> {
    pub(crate) strategy: &'ctx mut dyn RenderingTreeLayoutStrategy,

    pub(crate) tree: &'ctx mut RenderingTree,

    /// Whether the parent of this render object lays itself out based on the
    /// resulting size of this render object. This results in the parent being
    /// updated whenever this render object's layout is changed.
    ///
    /// This is `true` if the render object reads the sizing information of the
    /// children.
    pub(crate) parent_uses_size: &'ctx bool,

    pub(crate) relayout_boundary_id: &'ctx Option<RenderObjectId>,

    pub render_object_id: &'ctx RenderObjectId,

    pub children: &'ctx [RenderObjectId],
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

            tree: self.tree,

            children: self.children,
        }
    }

    pub fn iter_children_mut(&mut self) -> IterChildrenLayoutMut {
        IterChildrenLayoutMut {
            strategy: self.strategy,

            tree: self.tree,

            index: 0,

            relayout_boundary_id: self.relayout_boundary_id,

            children: self.children,
        }
    }

    pub fn parent_uses_size(&self) -> bool {
        *self.parent_uses_size
    }

    pub fn relayout_boundary_id(&self) -> Option<RenderObjectId> {
        *self.relayout_boundary_id
    }

    pub fn text_layout(&mut self) -> Option<&mut dyn RenderingTreeTextLayoutStrategy> {
        Some(
            self.tree
                .get_view_mut(*self.render_object_id)?
                .text_layout_mut(),
        )
    }
}
