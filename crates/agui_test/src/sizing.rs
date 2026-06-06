//! Conformance checks for the box-sizing contracts every [`RenderBox`] must satisfy.

use agui_core::{
    paint::{
        command::{PaintCommand, PaintShape},
        compositing::{Compositor, ContainerLayer, LayerHandle},
        peniko::kurbo::{self, Affine, Point, Shape},
        scene::Scene,
    },
    prelude::{element::*, render_object::*},
    provide::ProvideScope,
    test_harness::TestTaskRunner,
};
use typed_floats::{Positive, PositiveFinite};

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
/// [`extents`](Self::extents) and [`constraints`](Self::constraints), then [`run`](Self::run) it
/// against a widget.
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

    /// Checks `widget`'s render object against the configured samples.
    ///
    /// # Panics
    ///
    /// Panics on the first contract the render object breaks.
    pub fn run<W>(&self, widget: &W)
    where
        W: Widget,
        W::Render: RenderBox,
    {
        let mut tasks = TestTaskRunner::new();
        let provide = ProvideScope::new();
        let mut path = Vec::new();

        let element = {
            let mut scheduler = tasks.scheduler();
            let mut ctx = UpdateCtx::new(&mut scheduler, &mut path, provide.clone());

            widget.create_element(&mut ctx)
        };

        let make = || widget.create_render_object(&element);

        self.check_intrinsic_ordering(&make());

        for &constraints in &self.constraints {
            let measured = make().measure(constraints);
            let laid_out = make().layout(&mut LayoutCtx::detached(), constraints);

            assert!(
                measured == laid_out,
                "measure reported {measured:?} but layout produced {laid_out:?} under {constraints:?}; \
                 measure must report the size layout produces"
            );

            satisfies(measured, constraints);
            satisfies(laid_out, constraints);
        }

        self.check_baselines(make);
        self.check_stable_across_layout(make);
        self.check_layout_idempotent(make);

        if !self.allow_overflow {
            self.check_no_overflow_at_min(make);
        }

        if self.shrink_wraps_width {
            self.check_shrink_wraps(make, Axis::Horizontal);
        }

        if self.shrink_wraps_height {
            self.check_shrink_wraps(make, Axis::Vertical);
        }

        if self.width_independent_of_height {
            self.check_independent(&make(), Axis::Horizontal);
        }

        if self.height_independent_of_width {
            self.check_independent(&make(), Axis::Vertical);
        }

        if self.fills_width {
            self.check_fills(make, Axis::Horizontal);
        }

        if self.fills_height {
            self.check_fills(make, Axis::Vertical);
        }
    }

    fn check_baselines<R: RenderBox>(&self, make: impl Fn() -> R) {
        for &constraints in &self.constraints {
            for baseline in [TextBaseline::Alphabetic, TextBaseline::Ideographic] {
                let dry = make().measure_baseline(constraints, baseline);

                let mut render = make();
                render.layout(&mut LayoutCtx::detached(), constraints);
                let laid_out = render.distance_to_baseline(baseline);

                assert!(
                    dry == laid_out,
                    "measure_baseline({baseline:?}) reported {dry:?} but distance_to_baseline reported \
                     {laid_out:?} after layout under {constraints:?}; the dry baseline must match the \
                     laid-out baseline",
                );
            }
        }
    }

    fn check_stable_across_layout<R: RenderBox>(&self, make: impl Fn() -> R) {
        let mut render = make();

        let intrinsics_before = self.intrinsic_snapshot(&render);
        let measures_before = self.measure_snapshot(&render);

        // Laying out may populate internal state; the queries above must not change because of it.
        for &constraints in &self.constraints {
            render.layout(&mut LayoutCtx::detached(), constraints);
        }

        let intrinsics_after = self.intrinsic_snapshot(&render);
        let measures_after = self.measure_snapshot(&render);

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
    }

    fn check_layout_idempotent<R: RenderBox>(&self, make: impl Fn() -> R) {
        for &constraints in &self.constraints {
            let mut render = make();

            let first = render.layout(&mut LayoutCtx::detached(), constraints);
            let second = render.layout(&mut LayoutCtx::detached(), constraints);

            assert!(
                first == second,
                "layout produced {first:?} then {second:?} for the same constraints {constraints:?}; \
                 laying out twice must produce the same size",
            );
        }
    }

    fn intrinsic_snapshot(&self, render: &impl RenderBox) -> Vec<Option<PositiveFinite<f32>>> {
        let mut snapshot = Vec::new();

        for &extent in &self.extents {
            snapshot.push(render.min_intrinsic_width(extent));
            snapshot.push(render.max_intrinsic_width(extent));
            snapshot.push(render.min_intrinsic_height(extent));
            snapshot.push(render.max_intrinsic_height(extent));
        }

        snapshot
    }

    fn measure_snapshot(&self, render: &impl RenderBox) -> Vec<Size> {
        self.constraints
            .iter()
            .map(|&constraints| render.measure(constraints))
            .collect()
    }

    fn check_no_overflow_at_min<R: RenderBox>(&self, make: impl Fn() -> R) {
        for &extent in &self.extents {
            if !extent.get().is_finite() {
                continue;
            }

            if let Some(min_width) = make().min_intrinsic_width(extent) {
                let size = Size::new(min_width.get(), extent.get());
                Self::assert_paints_within(&make, BoxConstraints::tight(size));
            }

            if let Some(min_height) = make().min_intrinsic_height(extent) {
                let size = Size::new(extent.get(), min_height.get());
                Self::assert_paints_within(&make, BoxConstraints::tight(size));
            }
        }
    }

    fn assert_paints_within<R: RenderBox>(make: &impl Fn() -> R, constraints: BoxConstraints) {
        // Sub-pixel slack keeps a fill flush with the boundary from reading as overflow.
        const SLACK: f64 = 0.5;

        let mut render = make();
        let size = render.layout(&mut LayoutCtx::detached(), constraints);

        let layer = LayerHandle::new(ContainerLayer::new());
        PaintCtx::paint(&layer, |ctx| render.paint(ctx, Offset::ZERO));

        let Some(painted) = painted_bounds(&Compositor::compose(&layer)) else {
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

    fn check_intrinsic_ordering(&self, render: &impl RenderBox) {
        for &height in &self.extents {
            if let (Some(min), Some(max)) = (
                render.min_intrinsic_width(height),
                render.max_intrinsic_width(height),
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
                render.min_intrinsic_height(width),
                render.max_intrinsic_height(width),
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

    fn check_shrink_wraps<R: RenderBox>(&self, make: impl Fn() -> R, axis: Axis) {
        let (main, cross) = dims(axis);

        for &extent in &self.extents {
            if !extent.get().is_finite() {
                continue;
            }

            let expected = match axis {
                Axis::Horizontal => make().max_intrinsic_width(extent),
                Axis::Vertical => make().max_intrinsic_height(extent),
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

            let actual = make()
                .layout(&mut LayoutCtx::detached(), constraints)
                .extent(axis);

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

    fn check_independent(&self, render: &impl RenderBox, axis: Axis) {
        match axis {
            Axis::Horizontal => {
                assert_constant(&self.extents, "min_intrinsic_width", |e| {
                    render.min_intrinsic_width(e)
                });
                assert_constant(&self.extents, "max_intrinsic_width", |e| {
                    render.max_intrinsic_width(e)
                });
            }
            Axis::Vertical => {
                assert_constant(&self.extents, "min_intrinsic_height", |e| {
                    render.min_intrinsic_height(e)
                });
                assert_constant(&self.extents, "max_intrinsic_height", |e| {
                    render.max_intrinsic_height(e)
                });
            }
        }
    }

    fn check_fills<R: RenderBox>(&self, make: impl Fn() -> R, axis: Axis) {
        let main = dims(axis).0;

        for &constraints in &self.constraints {
            let max = constraints.max_axis(axis);

            if !max.get().is_finite() {
                continue;
            }

            let actual = make()
                .layout(&mut LayoutCtx::detached(), constraints)
                .extent(axis);

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

    use agui_core::{
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
        BoxSizingCheck::default().run(&TestBox::new(Size::new(20, 20)));
    }

    #[test]
    #[should_panic(expected = "must agree")]
    fn a_wrong_max_intrinsic_is_caught_for_a_shrink_wrapping_box() {
        BoxSizingCheck::new().shrink_wraps_width().run(&Liar);
    }

    #[test]
    #[should_panic(expected = "dry baseline must match")]
    fn a_dry_baseline_that_disagrees_with_the_laid_out_baseline_is_caught() {
        BoxSizingCheck::default().run(&Naughty {
            mode: Naughtiness::BadBaseline,
        });
    }

    #[test]
    #[should_panic(expected = "must not depend on layout state")]
    fn an_intrinsic_that_changes_after_layout_is_caught() {
        BoxSizingCheck::default().run(&Naughty {
            mode: Naughtiness::UnstableIntrinsic,
        });
    }

    #[test]
    #[should_panic(expected = "the same size")]
    fn a_layout_that_is_not_idempotent_is_caught() {
        BoxSizingCheck::default().run(&Naughty {
            mode: Naughtiness::GrowingLayout,
        });
    }

    #[test]
    #[should_panic(expected = "paint within itself")]
    fn a_box_that_paints_outside_its_minimum_is_caught() {
        BoxSizingCheck::default().run(&Naughty {
            mode: Naughtiness::OverflowsAtMin,
        });
    }

    #[test]
    fn an_overflowing_box_passes_when_overflow_is_allowed() {
        BoxSizingCheck::new().allow_overflow().run(&Naughty {
            mode: Naughtiness::OverflowsAtMin,
        });
    }

    /// A leaf that respects its constraints but reports a max intrinsic width its layout never
    /// produces, so only the shrink-wrap differential catches it.
    struct Liar;

    struct RenderLiar;

    impl Widget for Liar {
        type Element = ();

        type Render = RenderLiar;

        fn create_element(&self, _: &mut UpdateCtx) {}

        fn update(&self, (): &mut (), _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, (): &()) -> RenderLiar {
            RenderLiar
        }

        fn update_render_object(&self, (): &(), _: &mut RenderLiar) {}
    }

    impl RenderObject for RenderLiar {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
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
        type Element = ();

        type Render = RenderNaughty;

        fn create_element(&self, _: &mut UpdateCtx) {}

        fn update(&self, (): &mut (), _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, (): &()) -> RenderNaughty {
            RenderNaughty {
                mode: self.mode,
                laid_out: Cell::new(false),
                layouts: Cell::new(0),
            }
        }

        fn update_render_object(&self, (): &(), _: &mut RenderNaughty) {}
    }

    impl RenderObject for RenderNaughty {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
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
                Naughtiness::GrowingLayout if self.layouts.get() > 1 => 1.0,
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

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            if matches!(self.mode, Naughtiness::OverflowsAtMin) {
                let mut canvas = ctx.canvas();
                let brush = canvas.brush(Color::BLACK);
                canvas.fill(Fill::NonZero, brush, &(offset & Size::new(50, 50)));
            }
        }
    }
}
