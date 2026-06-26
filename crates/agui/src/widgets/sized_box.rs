use typed_floats::{Positive, PositiveFinite, as_const};

use crate::prelude::{element::*, render_object::*};

pub struct SizedBox<Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: Child,
}

impl Default for SizedBox<()> {
    fn default() -> Self {
        SizedBox {
            width: None,
            height: None,

            child: (),
        }
    }
}

impl SizedBox<()> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn shrink() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, 0.0)),
            height: Some(as_const!(Positive, f32, 0.0)),

            child: (),
        }
    }

    pub fn expand() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: (),
        }
    }

    /// Fixes the box's width.
    ///
    /// # Panics
    /// Panics if `width` is not a non-negative, finite number.
    pub fn width<T>(self, width: T) -> SizedBox<()>
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        SizedBox {
            width: Some(
                PositiveFinite::try_from(width)
                    .expect("invalid width given to SizedBox")
                    .into(),
            ),
            height: self.height,

            child: self.child,
        }
    }

    pub fn expand_width(self) -> SizedBox<()> {
        SizedBox {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: self.height,

            child: self.child,
        }
    }

    /// Fixes the box's height.
    ///
    /// # Panics
    /// Panics if `height` is not a non-negative, finite number.
    pub fn height<T>(self, height: T) -> SizedBox<()>
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        SizedBox {
            width: self.width,
            height: Some(
                PositiveFinite::try_from(height)
                    .expect("invalid height given to SizedBox")
                    .into(),
            ),

            child: self.child,
        }
    }

    pub fn expand_height(self) -> SizedBox<()> {
        SizedBox {
            width: self.width,
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: self.child,
        }
    }

    pub fn child<Child>(self, child: Child) -> SizedBox<Child> {
        SizedBox {
            width: self.width,
            height: self.height,

            child,
        }
    }
}

impl From<Size> for SizedBox<()> {
    fn from(size: Size) -> Self {
        Self {
            width: Some(
                PositiveFinite::<f32>::try_from(size.width)
                    .expect("width must be a positive finite number")
                    .into(),
            ),
            height: Some(
                PositiveFinite::<f32>::try_from(size.height)
                    .expect("height must be a positive finite number")
                    .into(),
            ),

            child: (),
        }
    }
}

impl<Child> Widget for SizedBox<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderSizedBox<Child::Render>>;

    type Render = RenderSizedBox<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderSizedBox {
                width: self.width,
                height: self.height,

                layout_scope: LayoutScope::detached(),
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if render.width != self.width || render.height != self.height {
            render.width = self.width;
            render.height = self.height;

            // A size change resizes the box itself, which its parent may have laid out around, so re-lay from
            // the box's own enclosing boundary, not the child boundary below it.
            ctx.mark_needs_layout(render.layout_scope);
        }

        element.update(ctx, self.child);
    }
}

pub struct RenderSizedBox<Child: ?Sized> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    /// The boundary the box is laid out under, captured each layout so a size-property change can re-lay it.
    layout_scope: LayoutScope,
    child: RenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderSizedBox<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderSizedBox<Child> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property_opt("width", self.width)
            .property_opt("height", self.height)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

/// Converts a `Positive` extent to `PositiveFinite`, returning `None` if infinite.
fn finite_extent(v: Option<Positive<f32>>) -> Option<PositiveFinite<f32>> {
    v.and_then(|x| PositiveFinite::try_from(x).ok())
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderSizedBox<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match finite_extent(self.width) {
            Some(w) => Some(w),
            None => self.child.min_intrinsic_width(height),
        }
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match finite_extent(self.width) {
            Some(w) => Some(w),
            None => self.child.max_intrinsic_width(height),
        }
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match finite_extent(self.height) {
            Some(h) => Some(h),
            None => self.child.min_intrinsic_height(width),
        }
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        match finite_extent(self.height) {
            Some(h) => Some(h),
            None => self.child.max_intrinsic_height(width),
        }
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child
            .measure(BoxConstraints::tight_for(self.width, self.height).enforce(constraints))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        let child_size = self.child.layout_and_get_size(
            ctx,
            BoxConstraints::tight_for(self.width, self.height).enforce(constraints),
        );

        self.child.parent_data = Some(child_size);

        child_size
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(
            BoxConstraints::tight_for(self.width, self.height).enforce(constraints),
            baseline,
        )
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let child_size = self
            .child
            .parent_data
            .as_ref()
            .expect("child has not been laid out");

        if !child_size.contains(position) {
            return HitTest::Pass;
        }

        self.child.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        test_fixtures::RecordingBox,
        test_harness::{RawWidget, TestCtx},
    };

    use super::*;

    #[test]
    fn results_in_correct_sizing() {
        let probe = RecordingBox::new();
        let laid_out = Rc::clone(&probe.laid_out);
        let (mut owner, view) = TestCtx::new().mount_view(
            SizedBox::new()
                .width(16)
                .height(48)
                .child(RawWidget::new(probe)),
        );
        view.resize(BoxConstraints::new(0, 128, 0, 128));
        owner.flush_layout();
        assert_eq!(
            laid_out.get(),
            Some(Size::new(16, 48)),
            "should use the given sizes"
        );

        let probe = RecordingBox::new();
        let laid_out = Rc::clone(&probe.laid_out);
        let (mut owner, view) = TestCtx::new().mount_view(
            SizedBox::new()
                .width(0)
                .height(16)
                .child(RawWidget::new(probe)),
        );
        view.resize(BoxConstraints::new(16, 128, 32, 128));
        owner.flush_layout();
        assert_eq!(
            laid_out.get(),
            Some(Size::new(16, 32)),
            "should clamp the given sizes to the constraints"
        );

        let probe = RecordingBox::new();
        let laid_out = Rc::clone(&probe.laid_out);
        let (mut owner, view) =
            TestCtx::new().mount_view(SizedBox::expand().child(RawWidget::new(probe)));
        view.resize(BoxConstraints::new(0, 128, 0, 128));
        owner.flush_layout();
        assert_eq!(
            laid_out.get(),
            Some(Size::new(128, 128)),
            "should expand to the largest size possible within the constraints"
        );
    }

    #[test]
    fn a_childless_box_lays_out_and_paints() {
        // A childless `SizedBox` has a `()` child render object; laying it out and painting it must run
        // `()`'s no-op `RenderBox` through its real (zero-sized) storage, not dereference an absent one.
        let (mut owner, view) = TestCtx::new().mount_view(SizedBox::new().width(16).height(48));
        view.resize(BoxConstraints::new(0, 128, 0, 128));
        owner.flush_layout();
        owner.flush_paint();
        let _frame = view.composite_frame();
    }
}
