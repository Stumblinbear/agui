use crate::{
    engine::rendering::context::RenderingLayoutContext,
    render::RenderObjectId,
    unit::{AsAny, Constraints, HitTest, HitTestResult, IntrinsicDimension, Offset, Rect, Size},
};

use super::canvas::{
    painter::{CanvasPainter, Head},
    Canvas,
};

mod context;
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

    /// The constraints given to the render object during its previous layout.
    constraints: Option<Constraints>,

    /// Tracks which render object should be used when this render object needs
    /// to have its layout updated.
    relayout_boundary_id: Option<RenderObjectId>,

    size: Size,
    offset: Offset,
}

impl RenderObject {
    pub fn new<R>(render_object: R) -> Self
    where
        R: RenderObjectImpl,
    {
        Self {
            render_object: smallbox!(render_object),

            constraints: None,

            relayout_boundary_id: None,

            size: Size::ZERO,
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

    /// Returns the constraints given to the render object during its previous layout.
    pub const fn constraints(&self) -> Option<Constraints> {
        self.constraints
    }

    pub const fn relayout_boundary_id(&self) -> Option<RenderObjectId> {
        self.relayout_boundary_id
    }

    pub const fn size(&self) -> Size {
        self.size
    }

    pub const fn offset(&self) -> Offset {
        self.offset
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn intrinsic_size<'ctx>(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext<'ctx>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        self.render_object
            .intrinsic_size(ctx, dimension, cross_extent)
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn layout<'ctx>(
        &mut self,
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

        // If we are a relayout boundary, we check if the constraints are the same as the previous
        // layout. If they are, we can reuse the size from the previous layout. However, even if the
        // constraints are the same, we still need to call layout so that the children can update
        // their target relayout boundary if it has changed.
        if is_relayout_boundary && relayout_boundary_id == self.relayout_boundary_id() {
            if let Some(previous_constraints) = self.constraints {
                if constraints == previous_constraints {
                    // If the render object is sized by its parent and the constraints are the same,
                    // we can reuse the size from the previous layout.
                    if self.render_object.is_sized_by_parent() {
                        tracing::trace!(
                            id = ?ctx.render_object_id,
                            size = ?self.size(),
                            "reusing size from previous layout for render object"
                        );

                        return self.size();
                    }
                }
            }
        }

        self.relayout_boundary_id = relayout_boundary_id;

        if self.constraints != Some(constraints) {
            self.constraints = Some(constraints);

            ctx.strategy.on_constraints_changed(
                RenderingLayoutContext {
                    tree: ctx.tree,

                    render_object_id: ctx.render_object_id,
                },
                self,
            );
        }

        let children = ctx
            .tree
            .as_ref()
            .get_children(*ctx.render_object_id)
            .cloned()
            .unwrap_or_default();

        let size = self.render_object.layout(
            &mut RenderObjectLayoutContext {
                strategy: ctx.strategy,

                tree: ctx.tree,

                parent_uses_size: ctx.parent_uses_size,

                // Make sure to propagate the relayout boundary if it is one.
                relayout_boundary_id: &relayout_boundary_id,

                render_object_id: ctx.render_object_id,

                children: &children,
            },
            constraints,
        );

        if size > constraints.constrain(size) {
            tracing::warn!(
                "render object returned a size that is larger than the constraints: {constraints:?}",
            );
        }

        // Check if the size actually changed before we mark it as changed.
        if self.size() != size {
            self.size = size;

            ctx.strategy.on_size_changed(
                RenderingLayoutContext {
                    tree: ctx.tree,

                    render_object_id: ctx.render_object_id,
                },
                self,
            );
        }

        ctx.strategy.on_laid_out(
            RenderingLayoutContext {
                tree: ctx.tree,

                render_object_id: ctx.render_object_id,
            },
            self,
        );

        size
    }

    #[tracing::instrument(level = "trace", skip(self, ctx))]
    pub fn hit_test(
        &self,
        ctx: RenderObjectContext,
        result: &mut HitTestResult,
        position: Offset,
    ) -> HitTest {
        let children = ctx
            .tree
            .get_children(*ctx.render_object_id)
            .map(|children| children.as_slice())
            .unwrap_or_default();

        let hit = self.render_object.hit_test(
            &mut RenderObjectHitTestContext {
                tree: ctx.tree,

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

    pub fn does_paint(&self) -> bool {
        self.render_object.does_paint()
    }

    pub fn paint_bounds(&self) -> Rect {
        self.render_object.paint_bounds(self.size())
    }

    #[tracing::instrument(level = "trace", skip(self))]
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
#[allow(clippy::needless_lifetimes)]
pub trait RenderObjectImpl: AsAny + Send {
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
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext<'ctx>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        if self.is_sized_by_parent() {
            return 0.0;
        }

        if ctx.has_children() {
            assert_eq!(
                ctx.child_count(),
                1,
                "render objects that do not defined an intrinsic_size function cannot have more than a single child"
            );

            // By default, we take the intrinsic size of the child.
            ctx.iter_children()
                .next()
                .expect("child render object missing while computing intrinsic size")
                .compute_intrinsic_size(dimension, cross_extent)
        } else {
            0.0
        }
    }

    fn layout<'ctx>(
        &self,
        ctx: &mut RenderObjectLayoutContext<'ctx>,
        constraints: Constraints,
    ) -> Size {
        if ctx.has_children() {
            assert_eq!(
                ctx.child_count(),
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

    /// Whether this render object is capable of painting.
    ///
    /// Returning `false` causes this render object to be skipped during painting,
    /// which can be useful for render objects that only exist to provide layout
    /// information.
    ///
    /// This should always return the same value for the lifetime of a given render
    /// object.
    fn does_paint(&self) -> bool {
        false
    }

    /// Returns the area covered by the paint of this box.
    ///
    /// This method calculates the total area affected by the paint of this box, which
    /// may differ from its size. The given size refers to the space allocated for the
    /// box during layout, but the paint area can extend beyond this, such as in cases
    /// where the box casts a shadow.
    ///
    /// The paint bounds provided by this method are relative to the box's local
    /// coordinate system.
    fn paint_bounds(&self, size: Size) -> Rect {
        Offset::ZERO & size
    }

    #[allow(unused_variables)]
    fn paint<'a>(&self, canvas: CanvasPainter<'a, Head<()>>) {}
}
