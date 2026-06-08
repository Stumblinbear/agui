use agui_core::{
    paint::peniko::{Color, Fill},
    prelude::{element::*, render_object::*},
};
use typed_floats::{Positive, PositiveFinite};

/// A leaf widget of a fixed size, optionally filling itself with a color.
///
/// It takes the size it was given regardless of the constraints it is laid out under, so a test can
/// place content of a known size under a layout widget and assert on the result. Give it a color to
/// have it paint a fill the size of its bounds, which a test can then find in the composited scene.
pub struct TestBox {
    size: Size,
    color: Option<Color>,
}

impl TestBox {
    pub fn new(size: Size) -> Self {
        Self { size, color: None }
    }

    /// Paints a fill of `color` covering the box's bounds.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
}

impl Widget for TestBox {
    type Element = LeafElement<RenderTestBox>;

    type Render = RenderTestBox;

    fn create(self, _: &mut UpdateCtx) -> (LeafElement<RenderTestBox>, Self::Render) {
        (
            LeafElement::new(),
            RenderTestBox {
                size: self.size,
                color: self.color,
            },
        )
    }

    fn update(
        self,
        _: &mut LeafElement<RenderTestBox>,
        render_object: &mut Self::Render,
        _: &mut UpdateCtx,
    ) {
        render_object.size = self.size;
        render_object.color = self.color;
    }
}

/// The render object of a [`TestBox`].
pub struct RenderTestBox {
    size: Size,
    color: Option<Color>,
}

impl RenderObject for RenderTestBox {
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

impl RenderBox for RenderTestBox {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.width.get()).ok()
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.width.get()).ok()
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.height.get()).ok()
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.size.height.get()).ok()
    }

    fn measure(&self, _: BoxConstraints) -> Size {
        self.size
    }

    fn layout(&mut self, _: &mut LayoutCtx, _: BoxConstraints) -> Size {
        self.size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, position: Offset) -> HitTest {
        let x = position.x.get();
        let y = position.y.get();

        if x >= 0.0 && y >= 0.0 && x <= self.size.width.get() && y <= self.size.height.get() {
            HitTest::Absorb
        } else {
            HitTest::Pass
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        if let Some(color) = self.color {
            let mut canvas = ctx.canvas();
            let brush = canvas.brush(color);
            canvas.fill(Fill::NonZero, brush, &(offset & self.size));
        }
    }
}

/// A leaf with independently chosen intrinsic sizes, for use as a child whose metrics a test controls.
///
/// A box that has children usually derives its own intrinsics from theirs, so put one of these under
/// the box being tested and the box's intrinsics become predictable. It lays out to its given size
/// clamped to the constraints, and reports that size as both its minimum and maximum intrinsic unless
/// [`min_intrinsic`](Self::min_intrinsic) or [`max_intrinsic`](Self::max_intrinsic) sets them apart,
/// which lets a test confirm a parent threads the minimum and maximum through to the right places.
pub struct IntrinsicBox {
    size: Size,
    min: Size,
    max: Size,
}

impl IntrinsicBox {
    pub fn new(size: Size) -> Self {
        Self {
            size,
            min: size,
            max: size,
        }
    }

    /// Sets the minimum intrinsic width and height the box reports.
    pub fn min_intrinsic(mut self, min: Size) -> Self {
        self.min = min;
        self
    }

    /// Sets the maximum intrinsic width and height the box reports.
    pub fn max_intrinsic(mut self, max: Size) -> Self {
        self.max = max;
        self
    }
}

impl Widget for IntrinsicBox {
    type Element = LeafElement<RenderIntrinsicBox>;

    type Render = RenderIntrinsicBox;

    fn create(self, _: &mut UpdateCtx) -> (LeafElement<RenderIntrinsicBox>, Self::Render) {
        (
            LeafElement::new(),
            RenderIntrinsicBox {
                size: self.size,
                min: self.min,
                max: self.max,
            },
        )
    }

    fn update(
        self,
        _: &mut LeafElement<RenderIntrinsicBox>,
        render_object: &mut Self::Render,
        _: &mut UpdateCtx,
    ) {
        render_object.size = self.size;
        render_object.min = self.min;
        render_object.max = self.max;
    }
}

/// The render object of an [`IntrinsicBox`].
pub struct RenderIntrinsicBox {
    size: Size,
    min: Size,
    max: Size,
}

impl RenderObject for RenderIntrinsicBox {
    fn mount(&mut self, _: &mut MountCtx) {}

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

impl RenderBox for RenderIntrinsicBox {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.min.width.get()).ok()
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.max.width.get()).ok()
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.min.height.get()).ok()
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        PositiveFinite::try_from(self.max.height.get()).ok()
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.size)
    }

    fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.size)
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
}
