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

                child: RelayoutRenderNode::new(child_render),
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

        render_object
            .child
            .with_object_mut(|child_obj| element.update(self.child, child_obj, ctx));
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

    child: RelayoutRenderNode<Child, Option<ChildParentData>>,
}

impl<Child: RenderBox> SingleChildRenderObject for RenderFractionallySizedBox<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        self.child.with_object(f)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        self.child.with_object_mut(f)
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

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property_opt("width_factor", self.width_factor)
            .property_opt("height_factor", self.height_factor)
            .property("alignment", self.alignment)
            .child(|d| self.child.describe(d))
            .finish()
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
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use agui_core::{
        prelude::{element::*, render_object::*},
        test_harness::TestCtx,
    };

    use typed_floats::as_const;

    use crate::center::Center;

    use super::*;

    type Captured = Rc<RefCell<Option<LayoutScope>>>;

    /// A single-child wrapper that counts its layouts, so a test can confirm it is not re-laid when only
    /// a boundary below it changes.
    struct Counter<Child> {
        layouts: Rc<Cell<usize>>,
        child: Child,
    }

    impl<Child> Widget for Counter<Child>
    where
        Child: Widget,
        Child::Render: RenderBox,
    {
        type Element = SingleChildElement<Child::Element, RenderCounter<Child::Render>>;
        type Render = RenderCounter<Child::Render>;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let (element, child_render) = SingleChildElement::new(self.child, ctx);
            (
                element,
                RenderCounter {
                    layouts: self.layouts,
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

    struct RenderCounter<Child> {
        layouts: Rc<Cell<usize>>,
        child: RenderNode<Child, Option<Size>>,
    }

    impl<Child> SingleChildRenderObject for RenderCounter<Child> {
        type Child = Child;

        fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
            f(&self.child.object)
        }

        fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
            f(&mut self.child.object)
        }
    }

    impl<Child: RenderBox> RenderObject for RenderCounter<Child> {
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

    impl<Child: RenderBox> RenderBox for RenderCounter<Child> {
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
            self.child.measure(constraints)
        }
        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            self.layouts.set(self.layouts.get() + 1);
            self.child.layout_and_get_size(ctx, constraints)
        }
        fn measure_baseline(
            &self,
            constraints: BoxConstraints,
            baseline: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            self.child.measure_baseline(constraints, baseline)
        }
        fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
            self.child.distance_to_baseline(baseline)
        }
        fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
            self.child.hit_test(result, position)
        }
        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.child.paint(ctx, offset);
        }
    }

    /// A leaf that counts its layouts and captures the scope it was laid out under, so a test can re-lay
    /// it the way a change inside it would.
    struct Probe {
        layouts: Rc<Cell<usize>>,
        captured: Captured,
    }

    impl Widget for Probe {
        type Element = LeafElement<RenderProbe>;
        type Render = RenderProbe;

        fn create(self, _: &mut UpdateCtx) -> (LeafElement<RenderProbe>, RenderProbe) {
            (
                LeafElement::new(),
                RenderProbe {
                    layouts: self.layouts,
                    captured: self.captured,
                },
            )
        }

        fn update(self, _: &mut LeafElement<RenderProbe>, _: &mut RenderProbe, _: &mut UpdateCtx) {}
    }

    struct RenderProbe {
        layouts: Rc<Cell<usize>>,
        captured: Captured,
    }

    impl RenderObject for RenderProbe {
        fn mount(&mut self, _: &mut MountCtx) {}
        fn unmount(&mut self, _: &mut MountCtx) {}
        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for RenderProbe {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }
        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }
        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }
        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }
        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }
        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            self.layouts.set(self.layouts.get() + 1);
            *self.captured.borrow_mut() = Some(ctx.scope().clone());

            constraints.smallest()
        }
        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
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

    /// Both factors size the child tightly even under the loose constraints Center hands down, so the
    /// boundary forms at the child and a change inside it does not re-lay anything above.
    #[test]
    fn a_factored_box_makes_its_child_a_relayout_boundary() {
        let outer = Rc::new(Cell::new(0));
        let probe_layouts = Rc::new(Cell::new(0));
        let captured: Captured = Rc::new(RefCell::new(None));

        let widget = Counter {
            layouts: Rc::clone(&outer),
            child: Center::new().child(
                FractionallySizedBox::new()
                    .width_factor(0.5_f32)
                    .height_factor(0.5_f32)
                    .child(Probe {
                        layouts: Rc::clone(&probe_layouts),
                        captured: Rc::clone(&captured),
                    }),
            ),
        };

        let (mut owner, view) = TestCtx::new().mount_view(widget);

        view.resize(BoxConstraints::new(0, 200, 0, 200));
        owner.flush_layout();
        assert_eq!(outer.get(), 1);
        assert_eq!(probe_layouts.get(), 1);

        // A change confined to the factored box's child marks the boundary it registered.
        captured
            .borrow()
            .clone()
            .expect("laid out once")
            .mark_needs_layout();

        owner.flush_layout();
        assert_eq!(
            probe_layouts.get(),
            2,
            "the tightly-factored child re-laid on its own"
        );
        assert_eq!(outer.get(), 1, "nothing above the boundary was re-laid");
    }

    #[test]
    fn sizes_child_to_a_fraction_of_the_constraints() {
        let mut tcx = TestCtx::new();
        let (_, mut render_object) = tcx.create(
            FractionallySizedBox::new()
                .width_factor(0.5_f32)
                .height_factor(1.0_f32),
        );

        let size = render_object.layout(&mut tcx.layout_ctx(), BoxConstraints::new(0, 200, 0, 100));

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
        let mut tcx = TestCtx::new();
        let (_, mut render_object) = tcx.create(
            FractionallySizedBox::new()
                .width_factor(0.5_f32)
                .height_factor(0.5_f32),
        );

        let size = render_object.layout(
            &mut tcx.layout_ctx(),
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
        let render_object = TestCtx::new().laid_out(
            FractionallySizedBox::new()
                .width_factor(0.5_f32)
                .height_factor(0.5_f32)
                .alignment(Alignment::TOP_LEFT),
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
    use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

    use super::FractionallySizedBox;
    use crate::sized_box::SizedBox;

    #[test]
    fn obeys_the_element_lifecycle() {
        ElementLifecycleCheck::new().single_child(|child| FractionallySizedBox::new().child(child));
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| FractionallySizedBox::new().child(SizedBox::new().width(20).height(10)));
    }
}
