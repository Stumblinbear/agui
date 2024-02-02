use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    render::{
        object::{
            layout_data::LayoutDataUpdate, RenderObject, RenderObjectContext,
            RenderObjectLayoutContext,
        },
        RenderObjectId,
    },
    unit::{Constraints, IntrinsicDimension, Offset, Size},
    util::tree::Tree,
};

pub struct IterChildrenLayout<'ctx> {
    pub(crate) index: usize,

    pub(crate) render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub(crate) children: &'ctx [RenderObjectId],
}

impl<'ctx> Iterator for IterChildrenLayout<'ctx> {
    type Item = ChildLayout<'ctx>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.children.len() {
            return None;
        }

        self.index += 1;

        Some(ChildLayout {
            render_object_tree: self.render_object_tree,

            index: self.index - 1,

            children: self.children,
        })
    }
}

pub struct ChildLayout<'ctx> {
    render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    index: usize,

    children: &'ctx [RenderObjectId],
}

impl ChildLayout<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn render_object_id(&self) -> RenderObjectId {
        self.children[self.index]
    }

    pub fn compute_intrinsic_size(&self, dimension: IntrinsicDimension, cross_extent: f32) -> f32 {
        let render_object_id = self.render_object_id();

        let render_object = self
            .render_object_tree
            .get(render_object_id)
            .expect("child render object missing during layout");

        render_object.intrinsic_size(
            RenderObjectContext {
                render_object_tree: self.render_object_tree,

                render_object_id: &render_object_id,
            },
            dimension,
            cross_extent,
        )
    }
}

pub struct IterChildrenLayoutMut<'ctx> {
    pub(crate) index: usize,

    pub(crate) render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub(crate) relayout_boundary_id: &'ctx Option<RenderObjectId>,

    pub(crate) children: &'ctx [RenderObjectId],

    pub(crate) constraints: &'ctx mut SecondaryMap<RenderObjectId, Constraints>,

    pub(crate) layout_changed: &'ctx mut SparseSecondaryMap<
        RenderObjectId,
        LayoutDataUpdate,
        BuildHasherDefault<FxHasher>,
    >,
}

// TODO: refactor to LendingIterator when possible
impl IterChildrenLayoutMut<'_> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<ChildLayoutMut> {
        if self.index >= self.children.len() {
            return None;
        }

        self.index += 1;

        Some(ChildLayoutMut {
            render_object_tree: self.render_object_tree,

            relayout_boundary_id: self.relayout_boundary_id,

            index: self.index - 1,

            children: self.children,

            constraints: self.constraints,

            layout_changed: self.layout_changed,
        })
    }
}

pub struct ChildLayoutMut<'ctx> {
    render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    relayout_boundary_id: &'ctx Option<RenderObjectId>,

    index: usize,

    children: &'ctx [RenderObjectId],

    constraints: &'ctx mut SecondaryMap<RenderObjectId, Constraints>,

    layout_changed: &'ctx mut SparseSecondaryMap<
        RenderObjectId,
        LayoutDataUpdate,
        BuildHasherDefault<FxHasher>,
    >,
}

impl ChildLayoutMut<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn render_object_id(&self) -> RenderObjectId {
        self.children[self.index]
    }

    pub fn compute_intrinsic_size(&self, dimension: IntrinsicDimension, cross_extent: f32) -> f32 {
        let render_object_id = self.render_object_id();

        let render_object = self
            .render_object_tree
            .get(render_object_id)
            .expect("child render object missing during layout");

        render_object.intrinsic_size(
            RenderObjectContext {
                render_object_tree: self.render_object_tree,

                render_object_id: &render_object_id,
            },
            dimension,
            cross_extent,
        )
    }

    /// Computes the layout of the child render object without returning its size.
    pub fn layout(&mut self, constraints: Constraints) {
        self.do_layout(constraints, false);
    }

    /// Computes the layout of the child render object and returns its resulting size.
    ///
    /// This binds the layout to the child element's layout, so that if the child's layout
    /// changes, this element will be laid out as well. If you do not need the sizing
    /// information of the child, use [layout()] instead.
    #[must_use = "If the size information is not needed, call layout() instead."]
    pub fn compute_layout(&mut self, constraints: Constraints) -> Size {
        self.do_layout(constraints, true)
    }

    fn do_layout(&mut self, constraints: Constraints, parent_uses_size: bool) -> Size {
        let render_object_id = self.render_object_id();

        let render_node = self
            .render_object_tree
            .get_node(render_object_id)
            .expect("child render object missing during layout");

        render_node.borrow().layout(
            &mut RenderObjectLayoutContext {
                render_object_tree: self.render_object_tree,

                parent_uses_size: &parent_uses_size,

                relayout_boundary_id: self.relayout_boundary_id,

                render_object_id: &render_object_id,

                children: render_node.children(),

                constraints: self.constraints,

                layout_changed: self.layout_changed,
            },
            constraints,
        )
    }

    pub fn set_offset(&mut self, offset: impl Into<Offset>) {
        let child_id = self.children[self.index];

        let child_render_object = self
            .render_object_tree
            .get(child_id)
            .expect("child render object missing during layout");

        let offset = offset.into();

        // is it worth even checking if this is equal there, or should
        // it just be set unconditionally and checked later?
        if child_render_object.offset() == offset {
            return;
        }

        self.layout_changed
            .entry(child_id)
            .unwrap()
            .or_default()
            .offset = Some(offset);
    }
}
