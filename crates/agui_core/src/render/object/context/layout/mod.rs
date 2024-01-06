use std::hash::BuildHasherDefault;

use crate::{
    element::ContextRenderObject,
    render::{
        object::{layout_data::LayoutDataUpdate, RenderObject},
        RenderObjectId,
    },
    util::tree::Tree,
};

mod iter;

pub use iter::*;
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

pub struct RenderObjectLayoutContext<'ctx> {
    pub(crate) render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub parent_uses_size: &'ctx bool,

    pub relayout_boundary_id: &'ctx Option<RenderObjectId>,

    pub render_object_id: &'ctx RenderObjectId,

    pub children: &'ctx [RenderObjectId],

    pub(crate) layout_changed: &'ctx mut SparseSecondaryMap<
        RenderObjectId,
        LayoutDataUpdate,
        BuildHasherDefault<FxHasher>,
    >,
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

            relayout_boundary_id: self.relayout_boundary_id,

            render_object_tree: self.render_object_tree,

            children: self.children,

            layout_changed: self.layout_changed,
        }
    }
}
