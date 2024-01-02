use rustc_hash::FxHashMap;

use crate::{
    render::{
        object::{LayoutResult, RenderObject, RenderObjectContext, RenderObjectLayoutContext},
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

    pub(crate) children: &'ctx [RenderObjectId],

    pub(crate) results: &'ctx mut FxHashMap<RenderObjectId, LayoutResult>,
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

            index: self.index - 1,

            children: self.children,

            results: self.results,
        })
    }
}

pub struct ChildLayoutMut<'ctx> {
    render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    index: usize,

    children: &'ctx [RenderObjectId],

    results: &'ctx mut FxHashMap<RenderObjectId, LayoutResult>,
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
    pub fn layout(&mut self, constraints: Constraints) -> Size {
        let render_object_id = self.render_object_id();

        let (render_object, children) = self
            .render_object_tree
            .get_node(render_object_id)
            .expect("child render object missing during layout")
            .into();

        render_object.layout(
            &mut RenderObjectLayoutContext {
                render_object_tree: self.render_object_tree,

                render_object_id: &render_object_id,

                children,

                results: self.results,
            },
            constraints,
        )
    }

    /// Computes the layout of the child render object and returns its resulting size.
    ///
    /// This binds the layout to the child element's layout, so that if the child's layout
    /// changes, this element will be laid out as well. If you do not need the sizing
    /// information of the child, use [layout()] instead.
    #[must_use = "If the size information is not needed, call layout() instead."]
    pub fn compute_layout(&mut self, constraints: Constraints) -> Size {
        self.results
            .entry(self.children[self.index])
            .or_default()
            .parent_uses_size = true;

        self.layout(constraints)
    }

    pub fn set_offset(&mut self, offset: impl Into<Offset>) {
        self.results
            .entry(self.children[self.index])
            .or_default()
            .offset = Some(offset.into());
    }
}
