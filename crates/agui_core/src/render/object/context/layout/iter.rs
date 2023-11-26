use crate::{
    plugin::Plugins,
    render::{RenderObject, RenderObjectContext, RenderObjectContextMut, RenderObjectId},
    unit::{Constraints, IntrinsicDimension, Offset, Size},
    util::tree::Tree,
};

pub struct IterChildrenLayout<'ctx> {
    pub(crate) index: usize,

    pub(crate) plugins: &'ctx Plugins,

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
            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            index: self.index - 1,

            children: self.children,
        })
    }
}

pub struct ChildLayout<'ctx> {
    plugins: &'ctx Plugins,

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
                plugins: self.plugins,

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

    pub(crate) plugins: &'ctx mut Plugins,

    pub(crate) render_object_tree: &'ctx mut Tree<RenderObjectId, RenderObject>,

    pub(crate) children: &'ctx [RenderObjectId],
    pub(crate) offsets: &'ctx mut [Offset],
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
            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            index: self.index - 1,

            children: self.children,
            offsets: self.offsets,
        })
    }
}

pub struct ChildLayoutMut<'ctx> {
    plugins: &'ctx mut Plugins,

    render_object_tree: &'ctx mut Tree<RenderObjectId, RenderObject>,

    index: usize,

    children: &'ctx [RenderObjectId],
    offsets: &'ctx mut [Offset],
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
                plugins: self.plugins,

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

        self.render_object_tree
            .with(render_object_id, |render_object_tree, render_object| {
                render_object.layout(
                    RenderObjectContextMut {
                        plugins: self.plugins,

                        render_object_tree,

                        render_object_id: &render_object_id,
                    },
                    constraints,
                )
            })
            .expect("child render object missing during layout")
    }

    /// Computes the layout of the child render object and returns its resulting size.
    ///
    /// This binds the layout to the child element's layout, so that if the child's layout
    /// changes, this element will be laid out as well. If you do not need the sizing
    /// information of the child, use [layout()] instead.
    #[must_use = "If the size information is not needed, call layout() instead."]
    pub fn compute_layout(&mut self, constraints: Constraints) -> Size {
        let render_object_id = self.render_object_id();

        self.render_object_tree
            .with(render_object_id, |render_object_tree, render_object| {
                render_object.set_parent_uses_size(true);

                render_object.layout(
                    RenderObjectContextMut {
                        plugins: self.plugins,

                        render_object_tree,

                        render_object_id: &render_object_id,
                    },
                    constraints,
                )
            })
            .expect("child render object missing during layout")
    }

    pub fn set_offset(&mut self, offset: impl Into<Offset>) {
        self.offsets[self.index] = offset.into();
    }
}
