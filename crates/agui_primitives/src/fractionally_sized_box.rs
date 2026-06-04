use typed_floats::{Positive, PositiveFinite};

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

/// A widget that sizes its child to a fraction of the space it is given.
///
/// On each axis that has a factor, the child is sized tightly to that fraction of the incoming
/// maximum; an axis with no factor passes the incoming constraints through. The box then takes its
/// child's size. The child is painted at the top-left.
pub struct FractionallySizedBox<Child> {
    width_factor: Option<PositiveFinite<f32>>,
    height_factor: Option<PositiveFinite<f32>>,

    child: Child,
}

impl Default for FractionallySizedBox<()> {
    fn default() -> Self {
        Self {
            width_factor: None,
            height_factor: None,

            child: (),
        }
    }
}

impl FractionallySizedBox<()> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn width_factor<T>(self, factor: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self {
            width_factor: Some(
                PositiveFinite::try_from(factor)
                    .expect("width factor must be a non-negative finite number"),
            ),

            ..self
        }
    }

    pub fn height_factor<T>(self, factor: T) -> Self
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self {
            height_factor: Some(
                PositiveFinite::try_from(factor)
                    .expect("height factor must be a non-negative finite number"),
            ),

            ..self
        }
    }

    pub fn child<Child>(self, child: Child) -> FractionallySizedBox<Child> {
        FractionallySizedBox {
            width_factor: self.width_factor,
            height_factor: self.height_factor,

            child,
        }
    }
}

impl<Child> Widget for FractionallySizedBox<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderFractionallySizedBox<Child::Render>;

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
        RenderFractionallySizedBox {
            width_factor: self.width_factor,
            height_factor: self.height_factor,

            child: RenderNode::new(element.create_render_object(&self.child)),
        }
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        // TODO(trevin): mark it for re-layout if the factors have changed
        render_object.width_factor = self.width_factor;
        render_object.height_factor = self.height_factor;

        element.update_render_object(&self.child, &mut render_object.child.object);
    }
}

pub struct RenderFractionallySizedBox<Child> {
    width_factor: Option<PositiveFinite<f32>>,
    height_factor: Option<PositiveFinite<f32>>,

    child: RenderNode<Child, Option<Size>>,
}

impl<Child> RenderFractionallySizedBox<Child> {
    fn inner_constraints(&self, constraints: Constraints) -> Constraints {
        let (min_width, max_width) = match self.width_factor {
            Some(factor) => {
                let width = PositiveFinite::try_from(constraints.max_width())
                    .expect("fractionally sized box received an unbounded width")
                    * factor;

                (width, width)
            }

            None => (constraints.min_width(), constraints.max_width()),
        };

        let (min_height, max_height) = match self.height_factor {
            Some(factor) => {
                let height = PositiveFinite::try_from(constraints.max_height())
                    .expect("fractionally sized box received an unbounded height")
                    * factor;

                (height, height)
            }

            None => (constraints.min_height(), constraints.max_height()),
        };

        Constraints::new(min_width, max_width, min_height, max_height)
    }
}

impl<Child> RenderObject for RenderFractionallySizedBox<Child>
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

impl<Child> RenderBox for RenderFractionallySizedBox<Child>
where
    Child: RenderBox,
{
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: Constraints) -> Size {
        constraints.constrain(self.child.measure(self.inner_constraints(constraints)))
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let child_size = self
            .child
            .layout_and_get_size(self.inner_constraints(constraints));

        self.child.parent_data = Some(child_size);

        constraints.constrain(child_size)
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child
            .measure_baseline(self.inner_constraints(constraints), baseline)
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

    use super::*;

    #[test]
    fn sizes_child_to_a_fraction_of_the_constraints() {
        let widget = FractionallySizedBox::new()
            .width_factor(0.5_f32)
            .height_factor(1.0_f32);

        let mut render_object =
            widget.create_render_object(&TestHarness::mount(&widget).root.element);

        let size = render_object.layout(Constraints::new(0, 200, 0, 100));

        assert_eq!(render_object.child.parent_data, Some(Size::new(100, 100)));
        assert_eq!(size, Size::new(100, 100));
    }
}
