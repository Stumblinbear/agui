use crate::{
    render::{
        binding::RenderView,
        object::layout_data::{LayoutData, LayoutDataUpdate},
        RenderObjectId,
    },
    unit::{AsAny, Constraints, HitTest, HitTestResult, IntrinsicDimension, Offset, Size},
};

use super::canvas::{
    painter::{CanvasPainter, Head},
    Canvas,
};

mod context;
pub mod layout_data;
mod render_box;

pub use context::*;
pub use render_box::*;
use smallbox::{smallbox, SmallBox};

/// The amount of space to allocate on the stack for a render object.
/// This is used to avoid indirection for small objects, which is a
/// very common case.
type RenderObjectSpace = smallbox::space::S4;

pub struct RenderObject {
    render_object: SmallBox<dyn RenderObjectImpl, RenderObjectSpace>,

    render_view: Option<RenderView>,

    layout_data: LayoutData,
}

impl RenderObject {
    pub fn new<R>(render_object: R) -> Self
    where
        R: RenderObjectImpl,
    {
        Self {
            render_object: smallbox!(render_object),

            render_view: None,

            layout_data: LayoutData::default(),
        }
    }

    pub fn is<R>(&self) -> bool
    where
        R: RenderObjectImpl,
    {
        (*self.render_object).as_any().is::<R>()
    }

    pub fn downcast_ref<R>(&self) -> Option<&R>
    where
        R: RenderObjectImpl,
    {
        (*self.render_object).as_any().downcast_ref::<R>()
    }

    pub fn downcast_mut<R>(&mut self) -> Option<&mut R>
    where
        R: RenderObjectImpl,
    {
        (*self.render_object).as_any_mut().downcast_mut::<R>()
    }

    pub fn render_object_name(&self) -> &str {
        (*self.render_object).short_type_name()
    }

    pub fn render_view(&self) -> Option<&RenderView> {
        self.render_view.as_ref()
    }

    pub(crate) fn set_render_view(&mut self, render_view: Option<RenderView>) {
        self.render_view = render_view;
    }

    pub fn relayout_boundary_id(&self) -> Option<RenderObjectId> {
        self.layout_data.relayout_boundary_id
    }

    pub fn parent_uses_size(&self) -> bool {
        self.layout_data.parent_uses_size
    }

    pub fn size(&self) -> Size {
        self.layout_data.size
    }

    pub fn offset(&self) -> Offset {
        self.layout_data.offset
    }

    pub(crate) fn apply_layout_data(&mut self, layout_update: LayoutDataUpdate) {
        layout_update.apply(&mut self.layout_data);
    }

    pub fn is_relayout_boundary(&self, constraints: Constraints) -> bool {
        !self.layout_data.parent_uses_size
            || constraints.is_tight()
            || self.render_object.is_sized_by_parent()
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
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
            &mut RenderObjectIntrinsicSizeContext {
                render_object_tree: ctx.render_object_tree,

                render_object_id: ctx.render_object_id,

                children,
            },
            dimension,
            cross_extent,
        )
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    pub fn layout<'ctx>(
        &self,
        ctx: &mut RenderObjectLayoutContext<'ctx>,
        constraints: Constraints,
    ) -> Size {
        let is_relayout_boundary = !ctx.parent_uses_size
            || constraints.is_tight()
            || self.render_object.is_sized_by_parent();

        let relayout_boundary_id = if is_relayout_boundary {
            Some(*ctx.render_object_id)
        } else {
            *ctx.relayout_boundary_id
        };

        let size = self.render_object.layout(
            &mut RenderObjectLayoutContext {
                render_object_tree: ctx.render_object_tree,

                parent_uses_size: ctx.parent_uses_size,

                // Make sure to propagate the relayout boundary if it is one.
                relayout_boundary_id: &relayout_boundary_id,

                render_object_id: ctx.render_object_id,

                children: ctx.children,

                layout_changed: ctx.layout_changed,
            },
            constraints,
        );

        debug_assert_eq!(
            size,
            constraints.constrain(size),
            "render object returned a size that is larger than the constraints"
        );

        // Check if the size actually changed before we mark it as changed.
        if self.size() != size {
            ctx.layout_changed
                .entry(*ctx.render_object_id)
                .unwrap()
                .or_default()
                .size = Some(size);
        }

        size
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    pub fn hit_test(
        &self,
        ctx: RenderObjectContext,
        result: &mut HitTestResult,
        position: Offset,
    ) -> HitTest {
        let children = ctx
            .render_object_tree
            .get_children(*ctx.render_object_id)
            .map(|children| children.as_slice())
            .unwrap_or_default();

        let hit = self.render_object.hit_test(
            &mut RenderObjectHitTestContext {
                render_object_tree: ctx.render_object_tree,

                render_object_id: ctx.render_object_id,
                size: &self.size(),

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

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn paint(&self) -> Canvas {
        let mut canvas = Canvas {
            size: self.size(),

            paints: Vec::default(),

            head: Vec::default(),
            children: Vec::default(),
            tail: None,
        };

        self.render_object.paint(CanvasPainter::begin(&mut canvas));

        canvas
    }
}

impl std::fmt::Debug for RenderObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DebugRenderObject(&'static str);

        impl std::fmt::Debug for DebugRenderObject {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct(self.0).finish_non_exhaustive()
            }
        }

        f.debug_struct("RenderObject")
            .field(
                "render_object",
                &DebugRenderObject((*self.render_object).short_type_name()),
            )
            .field("offset", &self.offset())
            .field("size", &self.size())
            .finish()
    }
}

#[cfg_attr(any(test, feature = "mocks"), mockall::automock)]
#[allow(clippy::disallowed_types)]
#[allow(clippy::needless_lifetimes)]
pub trait RenderObjectImpl: AsAny + Send + Sync {
    /// Whether the constraints are the only input to the sizing algorithm (i.e.
    /// given the same constraints, it will always return the same size regardless
    /// of other parameters, including children).
    ///
    /// Returning `false` is always correct, but returning `true` can be more
    /// efficient when computing the size of this render object because we don't
    /// need to recompute the size if the constraints don't change.
    fn is_sized_by_parent(&self) -> bool {
        false
    }

    fn intrinsic_size<'ctx>(
        &'ctx self,
        ctx: &mut RenderObjectIntrinsicSizeContext<'ctx>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        if self.is_sized_by_parent() {
            return 0.0;
        }

        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "render objects that do not defined an intrinsic_size function cannot have more than a single child"
            );

            let child_id = *ctx.children.first().unwrap();

            // By default, we take the intrinsic size of the child.
            ctx.render_object_tree
                .get(child_id)
                .expect("child render object missing while computing intrinsic size")
                .intrinsic_size(
                    RenderObjectContext {
                        render_object_tree: ctx.render_object_tree,

                        render_object_id: &child_id,
                    },
                    dimension,
                    cross_extent,
                )
        } else {
            0.0
        }
    }

    fn layout<'ctx>(
        &self,
        ctx: &mut RenderObjectLayoutContext<'ctx>,
        constraints: Constraints,
    ) -> Size {
        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "render objects that do not defined a layout function cannot have more than a single child"
            );

            // By default, we pass the constraints to the child and take its size.
            ctx.iter_children_mut()
                .next()
                .unwrap()
                .compute_layout(constraints)
        } else {
            constraints.biggest()
        }
    }

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
    fn paint<'a>(&self, canvas: CanvasPainter<'a, Head<()>>) {}
}
