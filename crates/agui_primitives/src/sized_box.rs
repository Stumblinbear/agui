use typed_floats::{Positive, PositiveFinite, as_const};

use agui_core::prelude::{element::*, render_object::*};

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
    type Element = SingleChildElement<Child::Element, RenderSizedBox<Child::Render>>;

    type Render = RenderSizedBox<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, child_render) = SingleChildElement::new(self.child, ctx);

        (
            element,
            RenderSizedBox {
                width: self.width,
                height: self.height,

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
        if render_object.width != self.width || render_object.height != self.height {
            render_object.width = self.width;
            render_object.height = self.height;

            render_object.layout_scope.mark_needs_layout();
        }

        render_object
            .child
            .with_object_mut(|child_obj| element.update(self.child, child_obj, ctx));
    }
}

pub struct RenderSizedBox<Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    layout_scope: LayoutScope,
    child: RelayoutRenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox> SingleChildRenderObject for RenderSizedBox<Child> {
    type Child = Child;

    fn with_child<R>(&self, f: impl FnOnce(&Child) -> R) -> R {
        self.child.with_object(f)
    }

    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Child) -> R) -> R {
        self.child.with_object_mut(f)
    }
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

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child
            .measure(BoxConstraints::tight_for(self.width, self.height).enforce(constraints))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = ctx.scope().clone();

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

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use agui_core::{
        paint::compositing::{LayerHandle, OffsetLayer},
        pipeline::{PipelineOwner, layout::BoundaryContent},
        prelude::{element::*, render_object::*},
        test_harness::with_ctx,
    };

    use crate::{center::Center, sized_box::SizedBox};

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

    #[test]
    fn results_in_correct_sizing() {
        let (_, mut render_object) =
            with_ctx(|ctx| SizedBox::new().width(16).height(48).create(ctx));
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(16, 48)),
            "should use the given sizes"
        );

        let (_, mut render_object) =
            with_ctx(|ctx| SizedBox::new().width(0).height(16).create(ctx));
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(16, 128, 32, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(16, 32)),
            "should ignore the given sizes and use the smallest size allowed by the constraints"
        );

        let (_, mut render_object) = with_ctx(|ctx| SizedBox::shrink().create(ctx));
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(0, 0)),
            "should shrink to the smallest size possible"
        );

        let (_, mut render_object) = with_ctx(|ctx| SizedBox::shrink().create(ctx));
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(10, 128, 20, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(10, 20)),
            "should shrink to the smallest size possible within the constraints"
        );

        let (_, mut render_object) = with_ctx(|ctx| SizedBox::expand().create(ctx));
        render_object.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::new(0, 128, 0, 128),
        );
        assert_eq!(
            render_object.child.parent_data.as_ref(),
            Some(&Size::new(128, 128)),
            "should expand to the largest size possible within the constraints"
        );
    }

    #[test]
    fn a_tight_sized_box_makes_its_child_a_relayout_boundary() {
        // Center hands the SizedBox loose constraints, so the SizedBox is not a boundary; the SizedBox
        // hands its own child tight constraints, so the boundary forms there. The outer counter confirms
        // the re-layout is isolated below it.
        let outer = Rc::new(Cell::new(0));
        let probe_layouts = Rc::new(Cell::new(0));
        let captured: Captured = Rc::new(RefCell::new(None));

        let widget = Counter {
            layouts: Rc::clone(&outer),
            child: Center::new().child(SizedBox::new().width(50).height(50).child(Probe {
                layouts: Rc::clone(&probe_layouts),
                captured: Rc::clone(&captured),
            })),
        };

        let (_, render) = with_ctx(|ctx| widget.create(ctx));
        let content: BoundaryContent = Rc::new(RefCell::new(render));
        let mut owner =
            PipelineOwner::new(Rc::clone(&content), LayerHandle::new(OffsetLayer::new()));

        owner.resize(BoxConstraints::new(0, 200, 0, 200));
        owner.flush_layout();
        assert_eq!(outer.get(), 1);
        assert_eq!(probe_layouts.get(), 1);

        // A change confined to the SizedBox's child marks the boundary it registered.
        captured
            .borrow()
            .clone()
            .expect("laid out once")
            .mark_needs_layout();

        owner.flush_layout();
        assert_eq!(
            probe_layouts.get(),
            2,
            "the tightly-constrained child re-laid on its own"
        );
        assert_eq!(outer.get(), 1, "nothing above the boundary was re-laid");
    }
}

#[cfg(test)]
mod harness {
    use agui_test::prelude::*;

    use super::SizedBox;

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default().run(|| SizedBox::new().width(16).height(48));
    }

    #[test]
    fn shrink_wraps_and_is_extent_independent() {
        BoxSizingCheck::new()
            .shrink_wraps_width()
            .shrink_wraps_height()
            .width_independent_of_height()
            .height_independent_of_width()
            .run(|| SizedBox::new().width(16).height(48));
    }

    #[test]
    fn expand_fills_its_constraints() {
        BoxSizingCheck::new()
            .fills_width()
            .fills_height()
            .run(SizedBox::expand);
    }

    #[test]
    fn lays_a_probed_box_out_to_its_given_size() {
        let probe = Probe::new();
        let mut tester = WidgetTester::mount(probe.wrap(SizedBox::new().width(16).height(48)));

        // Loose constraints let the box take its own size; a tight surface would force it to fill.
        tester.resize_with(BoxConstraints::loose(Size::new(128, 128)));
        tester.pump(Duration::ZERO);

        assert_eq!(probe.size(), Size::new(16, 48));
        assert_eq!(probe.layouts(), 1);
    }
}
