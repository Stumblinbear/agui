use crate::{
    engine::rendering::{
        context::RenderingLayoutContext, strategies::RenderingTreeLayoutStrategy, RenderingTree,
    },
    render::{
        object::{RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext},
        RenderObjectId,
    },
    unit::{Constraints, IntrinsicDimension, Offset, Size},
};

pub struct IterChildrenLayout<'ctx> {
    pub(crate) tree: &'ctx RenderingTree,

    pub(crate) index: usize,

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
            tree: self.tree,

            index: self.index - 1,

            children: self.children,
        })
    }
}

impl ExactSizeIterator for IterChildrenLayout<'_> {
    fn len(&self) -> usize {
        self.children.len() - self.index
    }
}

pub struct ChildLayout<'ctx> {
    tree: &'ctx RenderingTree,

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

        let render_object_node = self
            .tree
            .as_ref()
            .get_node(render_object_id)
            .expect("child render object missing during layout");

        render_object_node.borrow().intrinsic_size(
            &mut RenderObjectIntrinsicSizeContext {
                tree: self.tree,

                render_object_id: &render_object_id,

                children: render_object_node.children(),
            },
            dimension,
            cross_extent,
        )
    }
}

pub struct IterChildrenLayoutMut<'ctx> {
    pub(crate) strategy: &'ctx mut dyn RenderingTreeLayoutStrategy,

    pub(crate) tree: &'ctx mut RenderingTree,

    pub(crate) index: usize,

    pub(crate) relayout_boundary_id: &'ctx Option<RenderObjectId>,

    pub(crate) children: &'ctx [RenderObjectId],
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
            strategy: self.strategy,

            tree: self.tree,

            relayout_boundary_id: self.relayout_boundary_id,

            index: self.index - 1,

            children: self.children,
        })
    }
}

pub struct ChildLayoutMut<'ctx> {
    strategy: &'ctx mut dyn RenderingTreeLayoutStrategy,

    tree: &'ctx mut RenderingTree,

    relayout_boundary_id: &'ctx Option<RenderObjectId>,

    index: usize,

    children: &'ctx [RenderObjectId],
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

        let render_object_node = self
            .tree
            .as_ref()
            .get_node(render_object_id)
            .expect("child render object missing during layout");

        render_object_node.borrow().intrinsic_size(
            &mut RenderObjectIntrinsicSizeContext {
                tree: self.tree,

                render_object_id: &render_object_id,

                children: render_object_node.children(),
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

        self.tree
            .with(render_object_id, |tree, render_object| {
                // TODO: figure out how to not clone this
                let children = tree
                    .as_ref()
                    .get_children(render_object_id)
                    .cloned()
                    .unwrap_or_default();

                render_object.layout(
                    &mut RenderObjectLayoutContext {
                        strategy: self.strategy,

                        tree,

                        parent_uses_size: &parent_uses_size,

                        relayout_boundary_id: self.relayout_boundary_id,

                        render_object_id: &render_object_id,

                        children: &children,
                    },
                    constraints,
                )
            })
            .expect("child render object missing during layout")
    }

    pub fn set_offset(&mut self, offset: impl Into<Offset>) {
        let render_object_id = self.render_object_id();

        self.tree
            .with(render_object_id, |tree, render_object| {
                let offset = offset.into();

                // is it worth even checking if this is equal there, or should
                // it just be set unconditionally and checked later?
                if render_object.offset() == offset {
                    return;
                }

                self.strategy.on_offset_changed(
                    RenderingLayoutContext {
                        tree,

                        render_object_id: &render_object_id,
                    },
                    render_object,
                );
            })
            .expect("child render object missing while setting its offset")
    }
}
