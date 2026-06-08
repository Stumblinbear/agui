use typed_floats::{Positive, PositiveFinite};

use agui_core::prelude::{element::*, render_object::*};

/// A widget that centers its child within the space it is given.
///
/// The child is laid out under loosened constraints, so it may be any size up to the box. On each
/// bounded axis the box fills the space it was given and places the child in the middle; on an unbounded
/// axis the box shrinks to the child.
pub struct Center<Child> {
    child: Child,
}

impl Center<()> {
    pub fn new() -> Self {
        Self { child: () }
    }

    pub fn child<Child>(self, child: Child) -> Center<Child> {
        Center { child }
    }
}

impl Default for Center<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Child> Widget for Center<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderCenter<Child::Render>>;

    type Render = RenderCenter<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderCenter {
                child: RenderNode::new(child_render),
            },
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderCenter<Child> {
    child: RenderNode<Child, Option<ChildParentData>>,
}

impl<Child> SingleChildRenderObject for RenderCenter<Child> {
    type Child = Child;

    fn with_child<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderCenter<Child> {
    /// The box's size: it fills each bounded axis and shrinks to the child on an unbounded one.
    fn size_for(constraints: BoxConstraints, child_size: Size) -> Size {
        let max_width = constraints.max_width().get();
        let max_height = constraints.max_height().get();

        let width = if max_width.is_finite() {
            max_width
        } else {
            child_size.width.get()
        };
        let height = if max_height.is_finite() {
            max_height
        } else {
            child_size.height.get()
        };

        constraints.constrain(Size::new(width, height))
    }
}

impl<Child> RenderObject for RenderCenter<Child>
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

impl<Child> RenderBox for RenderCenter<Child>
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        let child_size = self.child.measure(constraints.loosen());

        Self::size_for(constraints, child_size)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        let child_size = self.child.layout_and_get_size(ctx, constraints.loosen());
        let size = Self::size_for(constraints, child_size);

        let offset = Alignment::CENTER.along_offset(Offset::new(
            size.width.get() - child_size.width.get(),
            size.height.get() - child_size.height.get(),
        ));

        self.child.parent_data = Some(ChildParentData { size, offset });

        size
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(constraints.loosen(), baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        let ChildParentData { size, offset } =
            self.child.parent_data.expect("child has not been laid out");

        if !size.contains(position) {
            return HitTest::Pass;
        }

        result.with_offset(offset, position, |result, transformed| {
            self.child.hit_test(result, transformed)
        })
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let child_offset = self
            .child
            .parent_data
            .expect("child has not been laid out")
            .offset;

        self.child.paint(ctx, offset + child_offset);
    }
}

#[cfg(test)]
mod tests {
    use agui_core::test_harness::with_ctx;

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn centers_the_child_within_a_bounded_box() {
        // A 50x50 child inside a 200x100 box sits at ((200-50)/2, (100-50)/2) = (75, 25).
        let widget = Center::new().child(SizedBox::new().width(50).height(50));

        let (_, mut render_object) = with_ctx(|ctx| widget.create(ctx));

        let size = render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 200, 0, 100),
        );

        assert_eq!(
            size,
            Size::new(200, 100),
            "the box fills its bounded constraints"
        );
        assert_eq!(
            render_object.child.parent_data,
            Some(ChildParentData {
                size: Size::new(200, 100),
                offset: Offset::new(75.0_f32, 25.0),
            })
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_test::prelude::*;

    use super::Center;
    use crate::sized_box::SizedBox;

    #[test]
    fn centers_a_box_within_the_surface() {
        let probe = Probe::new();
        let mut tester = WidgetTester::mount(
            Center::new().child(probe.wrap(TestBox::new(Size::new(20, 20)).color(Color::BLACK))),
        );

        tester.resize(Size::new(100, 100));
        tester.pump(Duration::ZERO);

        assert_eq!(probe.size(), Size::new(20, 20));
        assert_eq!(probe.offset(), Offset::new(40, 40));
        assert_eq!(probe.paints(), 1);
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default().run(|| Center::new().child(SizedBox::new().width(20).height(10)));
    }
}
