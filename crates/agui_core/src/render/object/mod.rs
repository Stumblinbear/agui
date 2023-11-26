use std::rc::Rc;

use crate::unit::{AsAny, Constraints, HitTest, HitTestResult, IntrinsicDimension, Offset, Size};

use super::{
    binding::RenderBinding,
    canvas::{
        painter::{CanvasPainter, Head},
        Canvas,
    },
};

mod context;
mod render_box;

pub use context::*;
pub use render_box::*;

slotmap::new_key_type! {
    pub struct RenderObjectId;
}

pub struct RenderObject {
    /// The constraints imposed on this render object by its parent
    constraints: Option<Constraints>,

    /// The current size of the render object.
    size: Option<Size>,

    /// Whether the parent of this render object lays itself out based on the
    /// resulting size of this render object. This results in the parent being
    /// updated whenever this render object's layout is changed.
    ///
    /// This is `true` if the render object reads the sizing information of the
    /// children.
    parent_uses_size: bool,

    /// Whether the constraints are the only input to the sizing algorithm (i.e.
    /// given the same constraints, it will always return the same size regardless
    /// of other parameters, including children).
    ///
    /// Returning `false` is always correct, but returning `true` can be more
    /// efficient when computing the size of this render object because we don't
    /// need to recompute the size if the constraints don't change.
    sized_by_parent: bool,

    offset: Offset,

    render_object: Box<dyn RenderObjectImpl>,
}

impl RenderObject {
    pub fn new<R>(render_object: R) -> Self
    where
        R: RenderObjectImpl,
    {
        Self {
            constraints: None,
            size: None,
            parent_uses_size: false,
            sized_by_parent: render_object.is_sized_by_parent(),
            offset: Offset::ZERO,

            render_object: Box::new(render_object),
        }
    }

    pub fn constraints(&self) -> Option<Constraints> {
        self.constraints
    }

    pub(crate) fn set_constraints(&mut self, constraints: Constraints) {
        self.constraints = Some(constraints);
    }

    pub fn size(&self) -> Option<Size> {
        self.size
    }

    pub(crate) fn set_parent_uses_size(&mut self, parent_uses_size: bool) {
        self.parent_uses_size = parent_uses_size;
    }

    pub fn is_relayout_boundary(&self) -> bool {
        !self.parent_uses_size
            || self.sized_by_parent
            || self
                .constraints
                .map_or(false, |constraints| constraints.is_tight())
    }

    pub fn offset(&self) -> Offset {
        self.offset
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

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    pub fn layout(&mut self, ctx: RenderObjectContextMut, constraints: Constraints) -> Size {
        self.constraints = Some(constraints);

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

        // The size of the render object may be larger than the constraints (currently, so we can determine intrinsic sizes),
        // so we have to ensure it's constrained, here.
        self.size = Some(constraints.constrain(size));

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

    #[tracing::instrument(level = "debug", skip(self, render_binding))]
    pub fn attach(&mut self, render_binding: &Rc<dyn RenderBinding>) {
        self.render_object.attach(render_binding);
    }

    #[tracing::instrument(level = "debug", skip(self, render_binding))]
    pub fn detatch(&mut self, render_binding: &Rc<dyn RenderBinding>) {
        self.render_object.detatch(render_binding);
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

        self.render_object
            .paint(CanvasPainter::<Head<()>>::begin(&mut canvas));

        canvas
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
        &'ctx mut self,
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

    /**
     * Called when the render object is attached to a renderer. This is where
     * the render object should create any resources it may need for painting.
     *
     * Note that it is possible for the render object to be attached to multiple
     * renderers at the same time, so it should generally not assume that it is
     * only attached to a single renderer even if this is often the case in
     * practice.
     */
    #[allow(unused_variables)]
    fn attach(&self, render_binding: &Rc<dyn RenderBinding>) {}

    /**
     * Called when the render object is detatched from a renderer. This case is
     * generally pretty rare, but it can happen if the render object is moved
     * from one renderer to another.
     *
     * This may not necessarily be called if the render object is dropped.
     */
    #[allow(unused_variables)]
    fn detatch(&self, render_binding: &Rc<dyn RenderBinding>) {}

    #[allow(unused_variables)]
    fn paint<'a>(&self, canvas: CanvasPainter<'a, Head<()>>) {}
}

impl std::fmt::Debug for Box<dyn RenderObjectImpl> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct((**self).short_type_name())
            .finish_non_exhaustive()
    }
}
