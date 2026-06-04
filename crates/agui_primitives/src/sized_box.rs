use typed_floats::{Positive, PositiveFinite, as_const};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::SingleChildElement,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::{MountCtx, RenderNode, RenderObject, box_layout::RenderBox},
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    widget::Widget,
};

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
}

impl SizedBox<()> {
    pub fn shrink() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, 0.0)),
            height: Some(as_const!(Positive, f32, 0.0)),

            child: (),
        }
    }
}

impl SizedBox<()> {
    pub fn expand() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: (),
        }
    }
}

impl SizedBox<()> {
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
}

impl SizedBox<()> {
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
}

impl SizedBox<()> {
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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderSizedBox<Child::Render>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        SingleChildElement::new(&self.child, ctx)
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        element.update(&self.child, &old.child, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        element.dispatch(&self.child, path, action)
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        RenderSizedBox {
            width: self.width,
            height: self.height,

            child: RenderNode::new(element.create_render_object(&self.child)),
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        // TODO(trevin): mark it for re-layout if these have changed
        render_object.width = self.width;
        render_object.height = self.height;

        element.update_render_object(&self.child, &mut render_object.child.object);
    }
}

pub struct RenderSizedBox<Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: RenderNode<Child, Option<Size>>,
}

impl<Child> RenderObject for RenderSizedBox<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }
}

/// Converts a `Positive` extent to `PositiveFinite`, returning `None` if infinite.
fn finite_extent(v: Option<Positive<f32>>) -> Option<PositiveFinite<f32>> {
    v.and_then(|x| PositiveFinite::try_from(x).ok())
}

impl<Child> RenderBox for RenderSizedBox<Child>
where
    Child: RenderBox,
{
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

    fn measure(&self, constraints: Constraints) -> Size {
        self.child
            .measure(Constraints::tight_for(self.width, self.height).enforce(constraints))
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let child_size = self.child.layout_and_get_size(
            Constraints::tight_for(self.width, self.height).enforce(constraints),
        );

        self.child.parent_data = Some(child_size);

        child_size
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(
            Constraints::tight_for(self.width, self.height).enforce(constraints),
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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use agui_core::test_harness::TestHarness;

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn results_in_correct_sizing() {
        let sized_box = SizedBox::new().width(16).height(48);
        let mut render_object =
            sized_box.create_render_object(&TestHarness::mount(&sized_box).root.element);
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(16, 48)),
            "should use the given sizes"
        );

        let sized_box = SizedBox::new().width(0).height(16);
        let mut render_object =
            sized_box.create_render_object(&TestHarness::mount(&sized_box).root.element);
        render_object.layout(Constraints::new(16, 128, 32, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(16, 32)),
            "should ignore the given sizes and use the smallest size allowed by the constraints"
        );

        let sized_box = SizedBox::shrink();
        let mut render_object =
            sized_box.create_render_object(&TestHarness::mount(&sized_box).root.element);
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(0, 0)),
            "should shrink to the smallest size possible"
        );

        let sized_box = SizedBox::shrink();
        let mut render_object =
            sized_box.create_render_object(&TestHarness::mount(&sized_box).root.element);
        render_object.layout(Constraints::new(10, 128, 20, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(10, 20)),
            "should shrink to the smallest size possible within the constraints"
        );

        let sized_box = SizedBox::expand();
        let mut render_object =
            sized_box.create_render_object(&TestHarness::mount(&sized_box).root.element);
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 128)),
            "should expand to the largest size possible within the constraints"
        );
    }
}
