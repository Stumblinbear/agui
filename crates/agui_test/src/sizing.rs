//! Conformance checks for the box-sizing contracts every [`RenderBox`] must satisfy.

use std::{cell::RefCell, rc::Rc, time::Duration};

use agui::{
    paint::{
        command::{PaintCommand, PaintShape},
        peniko::kurbo::{self, Affine, Point, Shape},
        scene::Scene,
    },
    prelude::{element::*, render_object::*},
};
use typed_floats::{Positive, PositiveFinite};

use crate::{WidgetTester, fixtures::IntrinsicBox};

/// Checks that a widget's render object obeys the box-sizing contracts.
///
/// By default it verifies the contracts every box must meet: intrinsic widths and heights ordered
/// `min <= max`, a size that stays within the constraints it was given, a `measure` that matches
/// `layout`, a dry baseline that matches the laid-out baseline, intrinsics and `measure` that a prior
/// layout does not change, a `layout` that produces the same size when run again, and content that
/// paints within its bounds at its minimum size (unless [`allow_overflow`](Self::allow_overflow) is
/// set). Per-axis opt-ins add expectations for a particular kind of box:
/// [`shrink_wraps_width`](Self::shrink_wraps_width) and
/// [`shrink_wraps_height`](Self::shrink_wraps_height) for a box that hugs its content,
/// [`fills_width`](Self::fills_width) and [`fills_height`](Self::fills_height) for one that fills its
/// space, and [`width_independent_of_height`](Self::width_independent_of_height) and
/// [`height_independent_of_width`](Self::height_independent_of_width) for one whose size on one axis is
/// unaffected by the other. Set the sampled extents and constraints with
/// [`extents`](Self::extents) and [`constraints`](Self::constraints), then run it against a box with
/// [`single_child`](Self::single_child) or [`leaf`](Self::leaf).
#[allow(clippy::struct_excessive_bools)]
pub struct BoxSizingCheck {
    extents: Vec<Positive<f32>>,
    constraints: Vec<BoxConstraints>,
    shrink_wraps_width: bool,
    shrink_wraps_height: bool,
    width_independent_of_height: bool,
    height_independent_of_width: bool,
    fills_width: bool,
    fills_height: bool,
    allow_overflow: bool,
}

impl Default for BoxSizingCheck {
    fn default() -> Self {
        Self {
            extents: vec![pos(0.0), pos(50.0), pos(200.0), pos(f32::INFINITY)],
            constraints: vec![
                BoxConstraints::tight(Size::new(0, 0)),
                BoxConstraints::tight(Size::new(50, 50)),
                BoxConstraints::tight(Size::new(120, 80)),
                BoxConstraints::loose(Size::new(200, 200)),
                BoxConstraints::new(20, 100, 30, 90),
                BoxConstraints::default(),
            ],
            shrink_wraps_width: false,
            shrink_wraps_height: false,
            width_independent_of_height: false,
            height_independent_of_width: false,
            fills_width: false,
            fills_height: false,
            allow_overflow: false,
        }
    }
}

impl BoxSizingCheck {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the cross-axis extents the intrinsic methods are queried at.
    pub fn extents(mut self, extents: impl IntoIterator<Item = Positive<f32>>) -> Self {
        self.extents = extents.into_iter().collect();
        self
    }

    /// Sets the constraints layout is run under.
    pub fn constraints(mut self, constraints: impl IntoIterator<Item = BoxConstraints>) -> Self {
        self.constraints = constraints.into_iter().collect();
        self
    }

    /// Turns off the default check that the box paints within its own bounds at its minimum size.
    /// Declare this for a box that paints outside itself on purpose, such as a drop shadow or an
    /// overflow indicator.
    pub fn allow_overflow(mut self) -> Self {
        self.allow_overflow = true;
        self
    }

    /// Also checks that the box takes its maximum intrinsic width when offered unbounded width with its
    /// height pinned. Declare this only for a box whose width comes from its content.
    ///
    /// # Criteria
    ///
    /// Place the box in a container that leaves its width free, then change what is inside it. If the
    /// box's width follows the size of its content, its width hugs its content. If its width comes from
    /// the room the container offers instead, it does not.
    ///
    /// # What this checks
    ///
    /// For each finite sampled extent, lays the box out with unbounded width and the extent as its
    /// height, then asserts the laid-out width equals [`RenderBox::max_intrinsic_width`] for that
    /// extent.
    pub fn shrink_wraps_width(mut self) -> Self {
        self.shrink_wraps_width = true;
        self
    }

    /// Also checks that the box takes its maximum intrinsic height when offered unbounded height with
    /// its width pinned. Declare this only for a box whose height comes from its content.
    ///
    /// # Criteria
    ///
    /// Place the box in a container that leaves its height free, then change what is inside it. If the
    /// box's height follows the size of its content, its height hugs its content. If its height comes
    /// from the room the container offers instead, it does not.
    ///
    /// # What this checks
    ///
    /// For each finite sampled extent, lays the box out with unbounded height and the extent as its
    /// width, then asserts the laid-out height equals [`RenderBox::max_intrinsic_height`] for that
    /// extent.
    pub fn shrink_wraps_height(mut self) -> Self {
        self.shrink_wraps_height = true;
        self
    }

    /// Also checks that the width the box wants never changes with the height available to it. Declare
    /// this only for a box whose width is unaffected by its height.
    ///
    /// # Criteria
    ///
    /// Place the box in a container and change only the container's height. If the box's width never
    /// moves in response, its width is independent of its height. If the width shifts, it is not.
    ///
    /// # What this checks
    ///
    /// Asserts [`RenderBox::min_intrinsic_width`] and [`RenderBox::max_intrinsic_width`] return the
    /// same value at every sampled extent.
    pub fn width_independent_of_height(mut self) -> Self {
        self.width_independent_of_height = true;
        self
    }

    /// Also checks that the height the box wants never changes with the width available to it. Declare
    /// this only for a box whose height is unaffected by its width.
    ///
    /// # Criteria
    ///
    /// Place the box in a container and change only the container's width. If the box's height never
    /// moves in response, its height is independent of its width. If the height shifts, it is not.
    ///
    /// # What this checks
    ///
    /// Asserts [`RenderBox::min_intrinsic_height`] and [`RenderBox::max_intrinsic_height`] return the
    /// same value at every sampled extent.
    pub fn height_independent_of_width(mut self) -> Self {
        self.height_independent_of_width = true;
        self
    }

    /// Also checks that the box takes the largest width allowed whenever its container permits a range.
    /// Declare this only for a box whose width fills the room it is given.
    ///
    /// # Criteria
    ///
    /// Place the box in a container and change only the container's width, leaving room to spare. If
    /// the box's width grows to match the container every time, its width fills. If it stays at the
    /// size of its content, or at a width you set, it does not.
    ///
    /// # What this checks
    ///
    /// For each sampled constraint whose maximum width is finite, asserts the laid-out width equals
    /// that maximum.
    pub fn fills_width(mut self) -> Self {
        self.fills_width = true;
        self
    }

    /// Also checks that the box takes the largest height allowed whenever its container permits a range.
    /// Declare this only for a box whose height fills the room it is given.
    ///
    /// # Criteria
    ///
    /// Place the box in a container and change only the container's height, leaving room to spare. If
    /// the box's height grows to match the container every time, its height fills. If it stays at the
    /// size of its content, or at a height you set, it does not.
    ///
    /// # What this checks
    ///
    /// For each sampled constraint whose maximum height is finite, asserts the laid-out height equals
    /// that maximum.
    pub fn fills_height(mut self) -> Self {
        self.fills_height = true;
        self
    }

    /// Runs the box-sizing battery against a box whose size or intrinsics derive from a child, wrapping a
    /// known-intrinsics [`IntrinsicBox`] the closure places inside the box.
    pub fn single_child<W>(&self, make: impl Fn(IntrinsicBox) -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
        W::Render: RenderBox + Sized + 'static,
    {
        self.check(|| make(IntrinsicBox::new(Size::new(40, 30))));
    }

    /// Runs the box-sizing battery against a box that sizes from itself rather than from a child.
    pub fn leaf<W>(&self, make: impl Fn() -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
        W::Render: RenderBox + Sized + 'static,
    {
        self.check(make);
    }

    /// Checks the render object built by `widget` against the configured samples.
    ///
    /// `widget` is a factory called once per render object the checks need, since building a render
    /// object consumes the widget that produced it.
    ///
    /// # Panics
    ///
    /// Panics on the first contract the render object breaks.
    fn check<W>(&self, widget: impl Fn() -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
        W::Render: RenderBox + Sized + 'static,
    {
        let config = Rc::new(self.config());
        let output = Rc::new(RefCell::new(Output::default()));

        let mut tester = WidgetTester::mount(SizingProbe {
            config: Rc::clone(&config),
            output: Rc::clone(&output),
            child: widget(),
        });
        tester.resize_with(BoxConstraints::loose(Size::new(500, 500)));
        tester.pump(Duration::ZERO);

        if !self.allow_overflow {
            self.check_no_overflow_at_min(&output.borrow(), &widget);
        }
    }

    fn config(&self) -> Config {
        Config {
            extents: self.extents.clone(),
            constraints: self.constraints.clone(),
            shrink_wraps_width: self.shrink_wraps_width,
            shrink_wraps_height: self.shrink_wraps_height,
            width_independent_of_height: self.width_independent_of_height,
            height_independent_of_width: self.height_independent_of_width,
            fills_width: self.fills_width,
            fills_height: self.fills_height,
        }
    }

    fn check_no_overflow_at_min<W>(&self, output: &Output, make: impl Fn() -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
        W::Render: RenderBox + Sized + 'static,
    {
        for &(extent, min_width) in &output.min_widths {
            if let Some(min_width) = min_width {
                let size = Size::new(min_width.get(), extent.get());
                Self::assert_paints_within(&make, BoxConstraints::tight(size));
            }
        }

        for &(extent, min_height) in &output.min_heights {
            if let Some(min_height) = min_height {
                let size = Size::new(extent.get(), min_height.get());
                Self::assert_paints_within(&make, BoxConstraints::tight(size));
            }
        }
    }

    fn assert_paints_within<W>(make: &impl Fn() -> W, constraints: BoxConstraints)
    where
        W: Widget + 'static,
        W::Element: 'static,
        W::Render: RenderBox + Sized + 'static,
    {
        // Sub-pixel slack keeps a fill flush with the boundary from reading as overflow.
        const SLACK: f64 = 0.5;

        // Always invoked with tight constraints, so the box that satisfies them takes this size.
        let size = constraints.smallest();

        let mut tester = WidgetTester::mount(make());
        tester.resize_with(constraints);
        tester.pump(Duration::ZERO);

        let Some(painted) = painted_bounds(&tester.composite_frame().rasterize()) else {
            return;
        };

        let bounds = kurbo::Rect::new(
            0.0,
            0.0,
            f64::from(size.width.get()),
            f64::from(size.height.get()),
        );

        assert!(
            painted.x0 >= bounds.x0 - SLACK
                && painted.y0 >= bounds.y0 - SLACK
                && painted.x1 <= bounds.x1 + SLACK
                && painted.y1 <= bounds.y1 + SLACK,
            "the box painted {painted:?}, outside its bounds {bounds:?}, when laid out at its minimum \
             size under {constraints:?}; a box must paint within itself unless it opts out with \
             allow_overflow()",
        );
    }
}

/// The configured samples and per-axis expectations, shared into the probe that runs the battery.
#[allow(clippy::struct_excessive_bools)]
struct Config {
    extents: Vec<Positive<f32>>,
    constraints: Vec<BoxConstraints>,
    shrink_wraps_width: bool,
    shrink_wraps_height: bool,
    width_independent_of_height: bool,
    height_independent_of_width: bool,
    fills_width: bool,
    fills_height: bool,
}

/// What the probe records from inside its layout for the checks that run afterward through a real paint:
/// the minimum intrinsic on each axis at each finite extent, keyed by that extent.
#[derive(Default)]
struct Output {
    min_widths: Vec<(Positive<f32>, Option<PositiveFinite<f32>>)>,
    min_heights: Vec<(Positive<f32>, Option<PositiveFinite<f32>>)>,
}

impl Config {
    /// Runs every layout-phase contract against `child`, the wired render object the probe holds, using
    /// the real `ctx` the pipeline handed the probe. Records the minimum intrinsics for the overflow
    /// check that runs later under a real paint.
    fn run_battery<C: RenderBox + ?Sized>(
        &self,
        child: &mut RenderNode<C>,
        ctx: &mut LayoutCtx,
        output: &mut Output,
    ) {
        self.check_intrinsic_ordering(child);

        // Snapshot the dry queries before anything is laid out, so the comparison after layout catches an
        // intrinsic or measure that depends on layout state.
        let intrinsics_before = self.intrinsic_snapshot(child);
        let measures_before = self.measure_snapshot(child);

        for &extent in &self.extents {
            if extent.get().is_finite() {
                output
                    .min_widths
                    .push((extent, child.min_intrinsic_width(extent)));
                output
                    .min_heights
                    .push((extent, child.min_intrinsic_height(extent)));
            }
        }

        self.check_layout_idempotent(child, ctx);

        for &constraints in &self.constraints {
            let measured = child.measure(constraints);
            let laid_out = child.layout_and_get_size(ctx, constraints);

            assert!(
                measured == laid_out,
                "measure reported {measured:?} but layout produced {laid_out:?} under {constraints:?}; \
                 measure must report the size layout produces"
            );

            satisfies(measured, constraints);
            satisfies(laid_out, constraints);
        }

        self.check_baselines(child, ctx);

        let intrinsics_after = self.intrinsic_snapshot(child);
        let measures_after = self.measure_snapshot(child);

        assert!(
            intrinsics_before == intrinsics_after,
            "an intrinsic changed after the box was laid out: before {intrinsics_before:?}, after \
             {intrinsics_after:?}; intrinsics must not depend on layout state",
        );

        assert!(
            measures_before == measures_after,
            "measure changed after the box was laid out: before {measures_before:?}, after \
             {measures_after:?}; measure must not depend on layout state",
        );

        if self.shrink_wraps_width {
            self.check_shrink_wraps(child, ctx, Axis::Horizontal);
        }

        if self.shrink_wraps_height {
            self.check_shrink_wraps(child, ctx, Axis::Vertical);
        }

        if self.width_independent_of_height {
            self.check_independent(child, Axis::Horizontal);
        }

        if self.height_independent_of_width {
            self.check_independent(child, Axis::Vertical);
        }

        if self.fills_width {
            self.check_fills(child, ctx, Axis::Horizontal);
        }

        if self.fills_height {
            self.check_fills(child, ctx, Axis::Vertical);
        }
    }

    fn check_baselines<C: RenderBox + ?Sized>(
        &self,
        child: &mut RenderNode<C>,
        ctx: &mut LayoutCtx,
    ) {
        for &constraints in &self.constraints {
            for baseline in [TextBaseline::Alphabetic, TextBaseline::Ideographic] {
                let dry = child.measure_baseline(constraints, baseline);

                child.layout_and_get_size(ctx, constraints);
                let laid_out = child.distance_to_baseline(baseline);

                assert!(
                    dry == laid_out,
                    "measure_baseline({baseline:?}) reported {dry:?} but distance_to_baseline reported \
                     {laid_out:?} after layout under {constraints:?}; the dry baseline must match the \
                     laid-out baseline",
                );
            }
        }
    }

    fn check_layout_idempotent<C: RenderBox + ?Sized>(
        &self,
        child: &mut RenderNode<C>,
        ctx: &mut LayoutCtx,
    ) {
        for &constraints in &self.constraints {
            let first = child.layout_and_get_size(ctx, constraints);
            let second = child.layout_and_get_size(ctx, constraints);

            assert!(
                first == second,
                "layout produced {first:?} then {second:?} for the same constraints {constraints:?}; \
                 laying out twice must produce the same size",
            );
        }
    }

    fn intrinsic_snapshot<C: RenderBox + ?Sized>(
        &self,
        child: &RenderNode<C>,
    ) -> Vec<Option<PositiveFinite<f32>>> {
        let mut snapshot = Vec::new();

        for &extent in &self.extents {
            snapshot.push(child.min_intrinsic_width(extent));
            snapshot.push(child.max_intrinsic_width(extent));
            snapshot.push(child.min_intrinsic_height(extent));
            snapshot.push(child.max_intrinsic_height(extent));
        }

        snapshot
    }

    fn measure_snapshot<C: RenderBox + ?Sized>(&self, child: &RenderNode<C>) -> Vec<Size> {
        self.constraints
            .iter()
            .map(|&constraints| child.measure(constraints))
            .collect()
    }

    fn check_intrinsic_ordering<C: RenderBox + ?Sized>(&self, child: &RenderNode<C>) {
        for &height in &self.extents {
            if let (Some(min), Some(max)) = (
                child.min_intrinsic_width(height),
                child.max_intrinsic_width(height),
            ) {
                assert!(
                    min <= max,
                    "min_intrinsic_width({h}) = {min} exceeds max_intrinsic_width({h}) = {max}; \
                     the minimum intrinsic must never exceed the maximum",
                    h = height.get(),
                    min = min.get(),
                    max = max.get(),
                );
            }
        }

        for &width in &self.extents {
            if let (Some(min), Some(max)) = (
                child.min_intrinsic_height(width),
                child.max_intrinsic_height(width),
            ) {
                assert!(
                    min <= max,
                    "min_intrinsic_height({w}) = {min} exceeds max_intrinsic_height({w}) = {max}; \
                     the minimum intrinsic must never exceed the maximum",
                    w = width.get(),
                    min = min.get(),
                    max = max.get(),
                );
            }
        }
    }

    fn check_shrink_wraps<C: RenderBox + ?Sized>(
        &self,
        child: &mut RenderNode<C>,
        ctx: &mut LayoutCtx,
        axis: Axis,
    ) {
        let (main, cross) = dims(axis);

        for &extent in &self.extents {
            if !extent.get().is_finite() {
                continue;
            }

            let expected = match axis {
                Axis::Horizontal => child.max_intrinsic_width(extent),
                Axis::Vertical => child.max_intrinsic_height(extent),
            };

            let Some(expected) = expected else {
                continue;
            };

            // Free the main axis; pin the cross axis to the extent.
            let constraints = match axis {
                Axis::Horizontal => {
                    BoxConstraints::new(0.0, f32::INFINITY, extent.get(), extent.get())
                }
                Axis::Vertical => {
                    BoxConstraints::new(extent.get(), extent.get(), 0.0, f32::INFINITY)
                }
            };

            let actual = child.layout_and_get_size(ctx, constraints).extent(axis);

            assert!(
                same(actual.get(), expected.get()),
                "max intrinsic {main} = {expected} but laying out with unbounded {main} and {cross} \
                 pinned to {e} produced {main} {actual}; a box whose {main} hugs its content must agree",
                e = extent.get(),
                expected = expected.get(),
                actual = actual.get(),
            );
        }
    }

    fn check_independent<C: RenderBox + ?Sized>(&self, child: &RenderNode<C>, axis: Axis) {
        match axis {
            Axis::Horizontal => {
                assert_constant(&self.extents, "min_intrinsic_width", |e| {
                    child.min_intrinsic_width(e)
                });
                assert_constant(&self.extents, "max_intrinsic_width", |e| {
                    child.max_intrinsic_width(e)
                });
            }
            Axis::Vertical => {
                assert_constant(&self.extents, "min_intrinsic_height", |e| {
                    child.min_intrinsic_height(e)
                });
                assert_constant(&self.extents, "max_intrinsic_height", |e| {
                    child.max_intrinsic_height(e)
                });
            }
        }
    }

    fn check_fills<C: RenderBox + ?Sized>(
        &self,
        child: &mut RenderNode<C>,
        ctx: &mut LayoutCtx,
        axis: Axis,
    ) {
        let main = dims(axis).0;

        for &constraints in &self.constraints {
            let max = constraints.max_axis(axis);

            if !max.get().is_finite() {
                continue;
            }

            let actual = child.layout_and_get_size(ctx, constraints).extent(axis);

            assert!(
                same(actual.get(), max.get()),
                "a box that fills its {main} must take the largest {main} offered, but took {actual} \
                 of {max} under {constraints:?}",
                actual = actual.get(),
                max = max.get(),
            );
        }
    }
}

/// The widget the check wraps the widget-under-test in. It presents its child to the pipeline like any
/// single-child box, and runs the box-sizing battery against it from inside its own layout.
struct SizingProbe<Child> {
    config: Rc<Config>,
    output: Rc<RefCell<Output>>,
    child: Child,
}

impl<Child> Widget for SizingProbe<Child>
where
    Child: Widget,
    Child::Render: RenderBox + Sized,
{
    type Element = SingleChildElement<Child::Element, RenderSizingProbe<Child::Render>>;

    type Render = RenderSizingProbe<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderSizingProbe {
                config: self.config,
                output: self.output,
                ran: false,
                child: RenderNode::new(()),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.update(ctx, self.child);
    }
}

/// The render object of a [`SizingProbe`]. It runs the battery against its wired child once, then forwards
/// layout to the child so it sits in the tree as a transparent box.
struct RenderSizingProbe<Child: ?Sized> {
    config: Rc<Config>,
    output: Rc<RefCell<Output>>,
    ran: bool,
    child: RenderNode<Child>,
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderSizingProbe<Child> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderSizingProbe<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderSizingProbe<Child> {
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
        if !self.ran {
            self.ran = true;
            let config = Rc::clone(&self.config);
            config.run_battery(&mut self.child, ctx, &mut self.output.borrow_mut());
        }

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

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}

/// Asserts `size` lies within `constraints` on both axes.
fn satisfies(size: Size, constraints: BoxConstraints) {
    let width = size.width.get();
    assert!(
        width >= constraints.min_width().get() && width <= constraints.max_width().get(),
        "size {size:?} does not satisfy {constraints:?}: width {width} is outside [{min}, {max}]",
        min = constraints.min_width().get(),
        max = constraints.max_width().get(),
    );

    let height = size.height.get();
    assert!(
        height >= constraints.min_height().get() && height <= constraints.max_height().get(),
        "size {size:?} does not satisfy {constraints:?}: height {height} is outside [{min}, {max}]",
        min = constraints.min_height().get(),
        max = constraints.max_height().get(),
    );
}

/// Asserts `intrinsic` reports the same value at every extent.
fn assert_constant(
    extents: &[Positive<f32>],
    name: &str,
    intrinsic: impl Fn(Positive<f32>) -> Option<PositiveFinite<f32>>,
) {
    let Some(&first) = extents.first() else {
        return;
    };

    let base = intrinsic(first);

    for &extent in extents {
        let here = intrinsic(extent);

        assert!(
            here == base,
            "{name} changes with the cross-axis extent: at {first} it is {base:?} but at {e} it is \
             {here:?}; a box declared extent-independent must not depend on it",
            first = first.get(),
            e = extent.get(),
        );
    }
}

fn pos(value: f32) -> Positive<f32> {
    Positive::try_from(value).expect("a sample extent must be non-negative")
}

/// The bounding box of everything `scene` paints, in the root coordinate space, or [`None`] if it
/// paints nothing. Drawing under a clip is intersected with it, so clipped overflow does not count.
fn painted_bounds(scene: &Scene) -> Option<kurbo::Rect> {
    let mut transform = Affine::IDENTITY;
    let mut transforms = Vec::new();

    let mut clip: Option<kurbo::Rect> = None;
    let mut clips = Vec::new();

    let mut painted: Option<kurbo::Rect> = None;

    for command in scene.flatten().commands() {
        match command {
            PaintCommand::PushTransform(next) => {
                transforms.push(transform);
                transform *= *next;
            }

            PaintCommand::PopTransform => {
                transform = transforms.pop().unwrap_or(Affine::IDENTITY);
            }

            PaintCommand::PushLayer { clip: shape, .. } => {
                let region = transform_bbox(transform, shape_bbox(shape));
                clips.push(clip);
                clip = Some(clip.map_or(region, |current| current.intersect(region)));
            }

            PaintCommand::PopLayer => {
                clip = clips.pop().flatten();
            }

            PaintCommand::Fill { shape, .. } | PaintCommand::Stroke { shape, .. } => {
                let mut bbox = transform_bbox(transform, shape_bbox(shape));

                if let Some(region) = clip {
                    bbox = bbox.intersect(region);
                }

                if bbox.width() > 0.0 && bbox.height() > 0.0 {
                    painted = Some(painted.map_or(bbox, |current| current.union(bbox)));
                }
            }

            // Glyph extents need font metrics this helper does not resolve; no sizing fixture paints text.
            PaintCommand::DrawGlyphs { .. } => {}

            PaintCommand::Embed { .. } => {
                unreachable!("flatten inlines every embed, so none remain")
            }
        }
    }

    painted
}

/// The bounding box of `rect` after `transform` is applied.
fn transform_bbox(transform: Affine, rect: kurbo::Rect) -> kurbo::Rect {
    let corners = [
        transform * Point::new(rect.x0, rect.y0),
        transform * Point::new(rect.x1, rect.y0),
        transform * Point::new(rect.x1, rect.y1),
        transform * Point::new(rect.x0, rect.y1),
    ];

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for point in corners {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    kurbo::Rect::new(min_x, min_y, max_x, max_y)
}

fn shape_bbox(shape: &PaintShape) -> kurbo::Rect {
    match shape {
        PaintShape::Rect(rect) => rect.bounding_box(),
        PaintShape::RoundedRect(rect) => rect.bounding_box(),
        PaintShape::Circle(circle) => circle.bounding_box(),
        PaintShape::Path(path) => path.bounding_box(),
    }
}

/// The names of an axis and its cross axis, for messages.
fn dims(axis: Axis) -> (&'static str, &'static str) {
    match axis {
        Axis::Horizontal => ("width", "height"),
        Axis::Vertical => ("height", "width"),
    }
}

// Intrinsics and layout run the same arithmetic, so a shrink-wrapping box must match to the bit.
#[allow(clippy::float_cmp)]
fn same(a: f32, b: f32) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use agui::{
        paint::peniko::{Color, Fill},
        prelude::{element::*, render_object::*},
    };
    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::BoxSizingCheck;
    use crate::fixtures::TestBox;

    #[test]
    #[should_panic(expected = "does not satisfy")]
    fn a_box_that_ignores_its_constraints_is_caught() {
        // TestBox takes its given size regardless of constraints, so a tight surface catches it.
        BoxSizingCheck::default().leaf(|| TestBox::new(Size::new(20, 20)));
    }

    #[test]
    #[should_panic(expected = "must agree")]
    fn a_wrong_max_intrinsic_is_caught_for_a_shrink_wrapping_box() {
        BoxSizingCheck::new().shrink_wraps_width().leaf(|| Liar);
    }

    #[test]
    #[should_panic(expected = "dry baseline must match")]
    fn a_dry_baseline_that_disagrees_with_the_laid_out_baseline_is_caught() {
        BoxSizingCheck::default().leaf(|| Naughty {
            mode: Naughtiness::BadBaseline,
        });
    }

    #[test]
    #[should_panic(expected = "must not depend on layout state")]
    fn an_intrinsic_that_changes_after_layout_is_caught() {
        BoxSizingCheck::default().leaf(|| Naughty {
            mode: Naughtiness::UnstableIntrinsic,
        });
    }

    #[test]
    #[should_panic(expected = "the same size")]
    fn a_layout_that_is_not_idempotent_is_caught() {
        BoxSizingCheck::default().leaf(|| Naughty {
            mode: Naughtiness::GrowingLayout,
        });
    }

    #[test]
    #[should_panic(expected = "paint within itself")]
    fn a_box_that_paints_outside_its_minimum_is_caught() {
        BoxSizingCheck::default().leaf(|| Naughty {
            mode: Naughtiness::OverflowsAtMin,
        });
    }

    #[test]
    fn an_overflowing_box_passes_when_overflow_is_allowed() {
        BoxSizingCheck::new().allow_overflow().leaf(|| Naughty {
            mode: Naughtiness::OverflowsAtMin,
        });
    }

    /// A leaf that respects its constraints but reports a max intrinsic width its layout never
    /// produces, so only the shrink-wrap differential catches it.
    struct Liar;

    struct RenderLiar;

    impl Widget for Liar {
        type Element = LeafElement<RenderLiar>;

        type Render = RenderLiar;

        fn create(self, _: &mut CreateCtx) -> LeafElement<RenderLiar> {
            LeafElement::new(RenderLiar)
        }

        fn update(self, _: &mut UpdateCtx, _: &mut LeafElement<RenderLiar>) {}
    }

    impl RenderObject for RenderLiar {
        fn build_semantics(&mut self, _: &mut SemanticsTreeBuilder<'_>) {}

        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>().finish()
        }
    }

    impl RenderBox for RenderLiar {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
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

        fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
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

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    #[derive(Clone, Copy)]
    enum Naughtiness {
        BadBaseline,
        UnstableIntrinsic,
        GrowingLayout,
        OverflowsAtMin,
    }

    /// A leaf that breaks one default contract on demand, to prove the default checks catch it.
    struct Naughty {
        mode: Naughtiness,
    }

    struct RenderNaughty {
        mode: Naughtiness,
        laid_out: Cell<bool>,
        layouts: Cell<u32>,
    }

    impl Widget for Naughty {
        type Element = LeafElement<RenderNaughty>;

        type Render = RenderNaughty;

        fn create(self, _: &mut CreateCtx) -> LeafElement<RenderNaughty> {
            LeafElement::new(RenderNaughty {
                mode: self.mode,
                laid_out: Cell::new(false),
                layouts: Cell::new(0),
            })
        }

        fn update(self, _: &mut UpdateCtx, _: &mut LeafElement<RenderNaughty>) {}
    }

    impl RenderObject for RenderNaughty {
        fn build_semantics(&mut self, _: &mut SemanticsTreeBuilder<'_>) {}

        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>().finish()
        }
    }

    impl RenderBox for RenderNaughty {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            // Reads state that layout sets, so the answer changes once the box has been laid out.
            match self.mode {
                Naughtiness::UnstableIntrinsic if self.laid_out.get() => {
                    Some(as_const!(PositiveFinite, f32, 20.0))
                }
                _ => Some(as_const!(PositiveFinite, f32, 10.0)),
            }
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 0.0))
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            Some(as_const!(PositiveFinite, f32, 10.0))
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.constrain(Size::new(10, 10))
        }

        fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            match self.mode {
                Naughtiness::UnstableIntrinsic => self.laid_out.set(true),
                Naughtiness::GrowingLayout => self.layouts.set(self.layouts.get() + 1),
                Naughtiness::BadBaseline | Naughtiness::OverflowsAtMin => {}
            }

            let grow = match self.mode {
                // Alternates each layout, so re-laying under the same loose constraints is never idempotent.
                Naughtiness::GrowingLayout if self.layouts.get() % 2 == 1 => 1.0,
                _ => 0.0,
            };

            constraints.constrain(Size::new(10.0 + grow, 10.0 + grow))
        }

        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            match self.mode {
                Naughtiness::BadBaseline => Some(as_const!(PositiveFinite, f32, 5.0)),
                _ => None,
            }
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            match self.mode {
                Naughtiness::BadBaseline => Some(as_const!(PositiveFinite, f32, 10.0)),
                _ => None,
            }
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn update_compositing_bits(&mut self) -> bool {
            false
        }

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            if matches!(self.mode, Naughtiness::OverflowsAtMin) {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(Color::BLACK);
                canvas.fill(Fill::NonZero, brush, &(offset & Size::new(50, 50)));
            }
        }
    }
}
