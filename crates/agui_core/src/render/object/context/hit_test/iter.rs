use glam::Mat4;

use crate::{
    plugin::Plugins,
    render::{RenderObject, RenderObjectContext, RenderObjectId},
    unit::{HitTest, HitTestResult, Offset},
    util::tree::Tree,
};

pub struct IterChildrenHitTest<'ctx> {
    pub(crate) front_index: usize,
    pub(crate) back_index: usize,

    pub(crate) plugins: &'ctx Plugins,

    pub(crate) render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    pub(crate) children: &'ctx [RenderObjectId],

    pub(crate) result: &'ctx mut HitTestResult,
}

// TODO: refactor to LendingIterator when possible
impl IterChildrenHitTest<'_> {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<ChildHitTest> {
        if self.front_index >= self.back_index {
            return None;
        }

        self.front_index += 1;

        Some(ChildHitTest {
            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            result: self.result,

            index: self.front_index - 1,

            children: self.children,
        })
    }

    pub fn next_back(&mut self) -> Option<ChildHitTest> {
        if self.front_index >= self.back_index {
            return None;
        }

        self.back_index -= 1;

        Some(ChildHitTest {
            plugins: self.plugins,

            render_object_tree: self.render_object_tree,

            index: self.back_index,

            children: self.children,

            result: self.result,
        })
    }
}

pub struct ChildHitTest<'ctx> {
    plugins: &'ctx Plugins,

    render_object_tree: &'ctx Tree<RenderObjectId, RenderObject>,

    index: usize,

    children: &'ctx [RenderObjectId],

    result: &'ctx mut HitTestResult,
}

impl ChildHitTest<'_> {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn render_object_id(&self) -> RenderObjectId {
        self.children[self.index]
    }

    pub fn render_object(&self) -> &RenderObject {
        let render_object_id = self.render_object_id();

        self.render_object_tree
            .get(render_object_id)
            .expect("child  render object missing during hit test")
    }

    pub fn offset(&self) -> Offset {
        let render_object_id = self.render_object_id();

        self.render_object_tree
            .get(render_object_id)
            .expect("child render object missing during hit test")
            .offset()
    }

    /// Check if the given position "hits" this widget or any of its descendants.
    ///
    /// The given position must be in the widget's local coordinate space, not the global
    /// coordinate space.
    pub fn hit_test(&mut self, position: Offset) -> HitTest {
        let render_object_id = self.render_object_id();

        let render_object = self
            .render_object_tree
            .get(render_object_id)
            .expect("child render object missing during hit test");

        render_object.hit_test(
            RenderObjectContext {
                plugins: self.plugins,

                render_object_tree: self.render_object_tree,

                render_object_id: &render_object_id,
            },
            self.result,
            position,
        )
    }

    pub fn with_transform(
        &mut self,
        transform: Mat4,
        position: Offset,
        func: impl FnOnce(&mut ChildHitTest<'_>, Offset) -> HitTest,
    ) -> HitTest {
        self.result.push_transform(transform);

        let transformed_position = transform.project_point3(position.into());
        let transformed_position = Offset::new(transformed_position.x, transformed_position.y);

        let hit = func(self, transformed_position);

        self.result.pop_transform();

        hit
    }

    pub fn hit_test_with_transform(&mut self, transform: Mat4, position: Offset) -> HitTest {
        self.with_transform(transform, position, |child, position| {
            child.hit_test(position)
        })
    }

    pub fn with_offset(
        &mut self,
        offset: Offset,
        position: Offset,
        func: impl FnOnce(&mut ChildHitTest<'_>, Offset) -> HitTest,
    ) -> HitTest {
        self.result.push_offset(offset);

        let child_offset = self.offset();

        let transformed_position = position - child_offset;

        let hit = func(self, transformed_position);

        self.result.pop_transform();

        hit
    }

    pub fn hit_test_with_offset(&mut self, offset: Offset, position: Offset) -> HitTest {
        self.with_offset(offset, position, |child, position| child.hit_test(position))
    }
}
