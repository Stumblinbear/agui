use typed_floats::{Positive, PositiveFinite};

use agui_core::prelude::{element::*, render_object::*};

/// A widget that sizes its child to a fraction of the space it is given, then positions it within.
///
/// On each axis that has a factor, the child is sized tightly to that fraction of the incoming
/// maximum; an axis with no factor passes the incoming constraints through. The box takes its child's
/// size, constrained to what it was given. When that leaves the box larger than the child, `alignment`
/// places the child within it, centered by default.
pub struct FractionallySizedBox<Child> {
    width_factor: Option<PositiveFinite<f32>>,
    height_factor: Option<PositiveFinite<f32>>,
    alignment: Alignment,

    child: Child,
}

impl Default for FractionallySizedBox<()> {
    fn default() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            alignment: Alignment::CENTER,

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

    /// Where to place the child when the box ends up larger than it. Defaults to
    /// [`Alignment::CENTER`].
    pub fn alignment(self, alignment: Alignment) -> Self {
        Self { alignment, ..self }
    }

    pub fn child<Child>(self, child: Child) -> FractionallySizedBox<Child> {
        FractionallySizedBox {
            width_factor: self.width_factor,
            height_factor: self.height_factor,
            alignment: self.alignment,

            child,
        }
    }
}

impl<Child> Widget for FractionallySizedBox<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderFractionallySizedBox<Child::Render>>;

    type Render = RenderFractionallySizedBox<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);
        (
            element,
            RenderFractionallySizedBox {
                width_factor: self.width_factor,
                height_factor: self.height_factor,
                alignment: self.alignment,

                layout_scope: LayoutScope::detached(),

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
        if render_object.width_factor != self.width_factor
            || render_object.height_factor != self.height_factor
            || render_object.alignment != self.alignment
        {
            render_object.width_factor = self.width_factor;
            render_object.height_factor = self.height_factor;
            render_object.alignment = self.alignment;

            render_object.layout_scope.mark_needs_layout();
        }

        element.update(self.child, &mut render_object.child.object, ctx);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderFractionallySizedBox<Child> {
    width_factor: Option<PositiveFinite<f32>>,
    height_factor: Option<PositiveFinite<f32>>,
    alignment: Alignment,

    layout_scope: LayoutScope,

    child: RenderNode<Child, Option<ChildParentData>>,
}

impl<Child> SingleChildRenderObject for RenderFractionallySizedBox<Child> {
    type Child = Child;

    fn with_child<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        f(&mut self.child.object)
    }
}

impl<Child> RenderFractionallySizedBox<Child> {
    fn inner_constraints(&self, constraints: BoxConstraints) -> BoxConstraints {
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

        BoxConstraints::new(min_width, max_width, min_height, max_height)
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.constrain(self.child.measure(self.inner_constraints(constraints)))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = ctx.scope().clone();

        let child_size = self
            .child
            .layout_and_get_size(ctx, self.inner_constraints(constraints));
        let size = constraints.constrain(child_size);

        let offset = self.alignment.along_offset(Offset::new(
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
        self.child
            .measure_baseline(self.inner_constraints(constraints), baseline)
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

    use super::*;

    #[test]
    fn sizes_child_to_a_fraction_of_the_constraints() {
        let (_, mut render_object) = with_ctx(|ctx| {
            FractionallySizedBox::new()
                .width_factor(0.5_f32)
                .height_factor(1.0_f32)
                .create(ctx)
        });

        let size = render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 200, 0, 100),
        );

        assert_eq!(size, Size::new(100, 100));
        assert_eq!(
            render_object.child.parent_data,
            Some(ChildParentData {
                size: Size::new(100, 100),
                offset: Offset::ZERO,
            }),
            "loose constraints leave the box at the child's size, so there is nothing to align"
        );
    }

    #[test]
    fn alignment_centers_the_child_when_the_box_is_forced_larger() {
        // A tight 200x200 forces the box to 200 while the factor sizes the child to 100; the default
        // center alignment then places the child at (50, 50).
        let (_, mut render_object) = with_ctx(|ctx| {
            FractionallySizedBox::new()
                .width_factor(0.5_f32)
                .height_factor(0.5_f32)
                .create(ctx)
        });

        let size = render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(200, 200, 200, 200),
        );

        assert_eq!(size, Size::new(200, 200));
        assert_eq!(
            render_object.child.parent_data,
            Some(ChildParentData {
                size: Size::new(200, 200),
                offset: Offset::new(50.0_f32, 50.0),
            })
        );
    }

    #[test]
    fn top_left_alignment_pins_the_child_to_the_origin() {
        let (_, mut render_object) = with_ctx(|ctx| {
            FractionallySizedBox::new()
                .width_factor(0.5_f32)
                .height_factor(0.5_f32)
                .alignment(Alignment::TOP_LEFT)
                .create(ctx)
        });

        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(200, 200, 200, 200),
        );

        assert_eq!(
            render_object.child.parent_data.unwrap().offset,
            Offset::ZERO
        );
    }
}

#[cfg(test)]
mod harness {
    use agui_test::prelude::*;

    use super::FractionallySizedBox;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| FractionallySizedBox::new().child(SizedBox::new().width(20).height(10)));
    }
}
