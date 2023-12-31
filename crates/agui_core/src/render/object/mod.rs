use crate::{
    render::RenderView,
    unit::{AsAny, Constraints, HitTest, HitTestResult, IntrinsicDimension, Offset, Size},
};

use super::canvas::{
    painter::{CanvasPainter, Head},
    Canvas,
};

mod context;
mod render_box;

pub use context::*;
pub use render_box::*;

slotmap::new_key_type! {
    pub struct RenderObjectId;
}

pub struct RenderObject {
    render_object: Box<dyn RenderObjectImpl>,

    render_view: Option<RenderView>,

    /// Tracks which render object should be used when this render object needs
    /// to have its layout updated.
    relayout_boundary_id: Option<RenderObjectId>,

    /// The current size of the render object.
    size: Option<Size>,

    /// Whether the parent of this render object lays itself out based on the
    /// resulting size of this render object. This results in the parent being
    /// updated whenever this render object's layout is changed.
    ///
    /// This is `true` if the render object reads the sizing information of the
    /// children.
    parent_uses_size: bool,

    offset: Offset,
}

impl RenderObject {
    pub fn new<R>(render_object: R) -> Self
    where
        R: RenderObjectImpl,
    {
        Self {
            render_object: Box::new(render_object),

            render_view: None,

            relayout_boundary_id: None,
            size: None,
            parent_uses_size: false,
            offset: Offset::ZERO,
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

    pub(crate) fn render_view(&self) -> Option<&RenderView> {
        self.render_view.as_ref()
    }

    pub(crate) fn set_render_view(&mut self, render_view: RenderView) {
        self.render_view = Some(render_view);
    }

    pub fn relayout_boundary_id(&self) -> Option<RenderObjectId> {
        self.relayout_boundary_id
    }

    pub(crate) fn set_relayout_boundary(&mut self, render_object_id: RenderObjectId) {
        self.relayout_boundary_id = Some(render_object_id);
    }

    pub fn size(&self) -> Option<Size> {
        self.size
    }

    pub fn does_parent_use_size(&self) -> bool {
        self.parent_uses_size
    }

    pub(crate) fn set_parent_uses_size(&mut self, parent_uses_size: bool) {
        self.parent_uses_size = parent_uses_size;
    }

    pub fn offset(&self) -> Offset {
        self.offset
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
                plugins: ctx.plugins,

                render_object_tree: ctx.render_object_tree,

                render_object_id: ctx.render_object_id,

                children,
            },
            dimension,
            cross_extent,
        )
    }

    pub(crate) fn is_relayout_boundary(&self, constraints: Constraints) -> bool {
        !self.does_parent_use_size()
            || constraints.is_tight()
            || self.render_object.is_sized_by_parent()
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    pub fn layout(&mut self, ctx: RenderObjectContextMut, constraints: Constraints) -> Size {
        let children = ctx
            .render_object_tree
            .get_children(*ctx.render_object_id)
            .cloned()
            .unwrap_or_default();

        let mut offsets = vec![Offset::ZERO; children.len()];

        let size = self.render_object.layout(
            &mut RenderObjectLayoutContext {
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

        debug_assert_eq!(
            size,
            constraints.constrain(size),
            "render object returned a size that is larger than the constraints"
        );

        self.size = Some(size);

        size
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
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

    #[tracing::instrument(level = "debug", skip(self))]
    pub fn paint(&self) -> Canvas {
        let size = self.size.expect("render object not laid out");

        let mut canvas = Canvas {
            size,

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
            .field("size", &self.size)
            .field("offset", &self.offset)
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
                .expect("child element missing while computing intrinsic size")
                .intrinsic_size(
                    RenderObjectContext {
                        plugins: ctx.plugins,

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
        &'ctx self,
        ctx: &mut RenderObjectLayoutContext<'ctx>,
        constraints: Constraints,
    ) -> Size {
        if !ctx.children.is_empty() {
            assert_eq!(
                ctx.children.len(),
                1,
                "render objects that do not defined a layout function cannot have more than a single child"
            );

            let child_id = *ctx.children.first().unwrap();

            // By default, we take the size of the child.
            ctx.render_object_tree
                .with(child_id, |render_object_tree, render_object| {
                    render_object.layout(
                        RenderObjectContextMut {
                            plugins: ctx.plugins,

                            render_object_tree,

                            render_object_id: &child_id,
                        },
                        constraints,
                    )
                })
                .expect("child render object missing during layout")
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
