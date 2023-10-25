use crate::unit::{AsAny, Constraints, HitTest, HitTestResult, IntrinsicDimension, Offset, Size};

use super::canvas::Canvas;

mod context;
mod render_box;

pub use context::*;
pub use render_box::*;

slotmap::new_key_type! {
    pub struct RenderObjectId;
}

pub struct RenderObject {
    size: Option<Size>,
    offset: Offset,

    render_object: Box<dyn RenderObjectImpl>,
}

impl RenderObject {
    pub fn new<R>(render_object: R) -> Self
    where
        R: RenderObjectImpl,
    {
        Self {
            size: None,
            offset: Offset::ZERO,

            render_object: Box::new(render_object),
        }
    }

    pub fn size(&self) -> Option<Size> {
        self.size
    }

    pub fn offset(&self) -> Offset {
        self.offset
    }

    pub fn downcast<R>(&self) -> Option<&R>
    where
        R: RenderObjectImpl,
    {
        (*self.render_object).as_any().downcast_ref::<R>()
    }

    pub fn render_object_name(&self) -> &str {
        self.render_object.render_object_name()
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn intrinsic_size(
        &self,
        ctx: RenderObjectContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        let children = ctx
            .render_object_tree
            .get_children(*ctx.render_object_id)
            .map(|children| children.as_slice())
            .unwrap_or_default();

        self.render_object.intrinsic_size(
            RenderObjectIntrinsicSizeContext {
                plugins: ctx.plugins,

                render_object_tree: ctx.render_object_tree,

                render_object_id: ctx.render_object_id,

                children,
            },
            dimension,
            cross_extent,
        )
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn layout(&mut self, ctx: RenderObjectContextMut, constraints: Constraints) -> Size {
        // TODO: technically if the constraints didn't change from the last layout, we shouldn't need to
        // recompute. Is this assumption correct? if the child hasn't rebuilt, will their layout _ever_ be
        // able to change?

        // TODO: if `render_object.layout` calls `.layout()` on its children, we need to update this render
        // object when the child indicates that its layout has changed. If they don't call it, we can skip
        // layout on render objects that haven't changed.

        let children = ctx
            .render_object_tree
            .get_children(*ctx.render_object_id)
            .cloned()
            .unwrap_or_default();

        let mut offsets = vec![Offset::ZERO; children.len()];

        let size = self.render_object.layout(
            RenderObjectLayoutContext {
                plugins: ctx.plugins,

                render_object_tree: ctx.render_object_tree,

                render_object_id: ctx.render_object_id,

                children: &children,

                offsets: &mut offsets,
            },
            constraints,
        );

        for (child_id, offset) in children.iter().zip(offsets) {
            ctx.render_object_tree
                .get_mut(*child_id)
                .expect("child render object missing during layout")
                .offset = offset;
        }

        // The size of the render object may be larger than the constraints (currently, so we can determine intrinsic sizes),
        // so we have to ensure it's constrained, here.
        self.size = Some(constraints.constrain(size));

        size
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn hit_test(
        &self,
        ctx: RenderObjectContext,
        result: &mut HitTestResult,
        position: Offset,
    ) -> HitTest {
        let Some(size) = self.size else {
            tracing::warn!("cannot hit test an element before layout");
            return HitTest::Pass;
        };

        let children = ctx
            .render_object_tree
            .get_children(*ctx.render_object_id)
            .map(|children| children.as_slice())
            .unwrap_or_default();

        let hit = self.render_object.hit_test(
            &mut RenderObjectHitTestContext {
                plugins: ctx.plugins,

                render_object_tree: ctx.render_object_tree,

                render_object_id: ctx.render_object_id,
                size: &size,

                children,

                result,
            },
            position,
        );

        if hit == HitTest::Absorb {
            result.add(*ctx.render_object_id);
        }

        hit
    }

    pub fn paint(&self) -> Option<Canvas> {
        let size = self.size.expect("render object not laid out");

        self.render_object.paint(size)
    }
}

impl std::fmt::Debug for RenderObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderObject")
            .field("size", &self.size)
            .field("offset", &self.offset)
            .field("render_object", &self.render_object)
            .finish()
    }
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
#[allow(clippy::disallowed_types)]
#[allow(clippy::needless_lifetimes)]
pub trait RenderObjectImpl: AsAny + Send + Sync {
    fn render_object_name(&self) -> &'static str;

    fn intrinsic_size<'ctx>(
        &self,
        ctx: RenderObjectIntrinsicSizeContext<'ctx>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32;

    fn layout<'ctx>(
        &mut self,
        ctx: RenderObjectLayoutContext<'ctx>,
        constraints: Constraints,
    ) -> Size;

    fn hit_test<'ctx>(
        &self,
        ctx: &'ctx mut RenderObjectHitTestContext<'ctx>,
        position: Offset,
    ) -> HitTest {
        if ctx.size.contains(position) {
            while let Some(mut child) = ctx.iter_children().next_back() {
                let offset = position - child.offset();

                if child.hit_test_with_offset(offset, position) == HitTest::Absorb {
                    return HitTest::Absorb;
                }
            }
        }

        HitTest::Pass
    }

    #[allow(unused_variables)]
    fn paint(&self, size: Size) -> Option<Canvas> {
        None
    }
}

impl std::fmt::Debug for Box<dyn RenderObjectImpl> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.render_object_name())
            .finish_non_exhaustive()
    }
}
