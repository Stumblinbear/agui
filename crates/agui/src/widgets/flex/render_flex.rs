use std::any::Any;

use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

use super::{
    CrossAxisAlignment, FlexData, FlexFit, FlexParentData, MainAxisAlignment, MainAxisSize,
    VerticalDirection,
};

/// The fixed configuration of a flex, set by the [`Row`](super::Row) or [`Column`](super::Column) that builds
/// it and read each layout.
#[derive(Clone, Copy, PartialEq)]
pub struct FlexConfig {
    pub direction: Axis,
    pub main_axis_size: MainAxisSize,
    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,
    pub text_direction: Option<TextDirection>,
}

/// A flat, mutable view of a flex's child render nodes, addressed by index.
// The layout core runs over this trait object so it compiles once, not once per concrete child-list shape.
trait FlexChildren {
    fn len(&self) -> usize;
    fn child(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox>;
}

impl<C: RenderChildren<ChildData = ()>> FlexChildren for C {
    fn len(&self) -> usize {
        RenderChildren::len(self)
    }

    fn child(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox> {
        self.get_mut(index)
    }
}

/// The render object of a [`Row`](super::Row) or [`Column`](super::Column): lays a run of children along its
/// main axis, sharing the free space among the flexible ones, and aligns the rest.
pub struct RenderFlex<Children> {
    config: FlexConfig,

    layout_scope: LayoutScope,
    children: Children,

    // Each child's offset and size from the last layout, kept for paint, hit-testing, and semantics, which
    // run after layout when the child list is no longer walked for sizing.
    offsets: Vec<Offset>,
    child_sizes: Vec<Size>,

    size: Size,
}

impl<Children> RenderFlex<Children> {
    pub fn new(config: FlexConfig, children: Children) -> Self {
        Self {
            config,
            layout_scope: LayoutScope::detached(),
            children,
            offsets: Vec::new(),
            child_sizes: Vec::new(),
            size: Size::ZERO,
        }
    }

    /// Replaces the configuration and re-lays the flex when it changes.
    pub fn set_config(&mut self, ctx: &mut UpdateCtx, config: FlexConfig) {
        if self.config != config {
            self.config = config;
            ctx.mark_needs_layout(self.layout_scope);
        }
    }
}

impl<Children: RenderChildren<ChildData = ()> + 'static> MultiChildRenderObject
    for RenderFlex<Children>
{
    type Children = Children;

    fn children_mut(&mut self) -> &mut Children {
        &mut self.children
    }

    fn layout_scope(&self) -> LayoutScope {
        self.layout_scope
    }
}

impl<Children: RenderChildren<ChildData = ()> + 'static> RenderObject for RenderFlex<Children> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        for index in 0..self.children.len() {
            let offset = self.offsets[index];
            s.with_offset(offset, |s| self.children.get_mut(index).build_semantics(s));
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl<Children: RenderChildren<ChildData = ()> + 'static> RenderBox for RenderFlex<Children> {
    fn min_intrinsic_width(&self, _height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        let main = self.config.direction;
        let cross = main.flip();
        let max_main = constraints.max_axis(main).get();
        let cross_max = constraints.max_axis(cross);
        let can_flex = max_main.is_finite();
        let stretch = matches!(
            self.config.cross_axis_alignment,
            CrossAxisAlignment::Stretch
        );

        let mut allocated = 0.0f32;
        let mut total_flex = 0.0f32;
        let mut cross_size = 0.0f32;

        for index in 0..self.children.len() {
            let data = flex_data(self.children.get(index).parent_data());
            if data.flex > 0.0 {
                total_flex += data.flex;
                continue;
            }
            let child = child_constraints(main, 0.0, f32::INFINITY, stretch, cross_max);
            let size = self.children.get(index).measure(child);
            allocated += size.extent(main).get();
            cross_size = cross_size.max(size.extent(cross).get());
        }

        let free = (max_main - allocated).max(0.0);
        let per_flex = if total_flex > 0.0 && can_flex {
            free / total_flex
        } else {
            0.0
        };
        for index in 0..self.children.len() {
            let data = flex_data(self.children.get(index).parent_data());
            if data.flex <= 0.0 {
                continue;
            }
            let max_extent = per_flex * data.flex;
            let min_extent = if data.fit == FlexFit::Tight {
                max_extent
            } else {
                0.0
            };
            let child = child_constraints(main, min_extent, max_extent, stretch, cross_max);
            let size = self.children.get(index).measure(child);
            allocated += size.extent(main).get();
            cross_size = cross_size.max(size.extent(cross).get());
        }

        let main_size = own_main(self.config.main_axis_size, can_flex, max_main, allocated);
        size_for(
            main,
            constraints.constrain_axis(main, main_size).get(),
            own_cross(constraints, cross, stretch, cross_max, cross_size),
        )
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        let size = flex_layout(
            &mut self.children,
            ctx,
            self.config,
            constraints,
            self.layout_scope,
            &mut self.offsets,
            &mut self.child_sizes,
        );

        self.size = size;
        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        if !self.size.contains(position) {
            return HitTest::Pass;
        }

        // Front-to-back: the last child painted sits on top, so it is tested first.
        for index in (0..self.children.len()).rev() {
            let offset = self.offsets[index];
            let size = self.child_sizes[index];

            if !size.contains(position - offset) {
                continue;
            }

            let hit = result.with_offset(offset, position, |result, transformed| {
                self.children.get(index).hit_test(result, transformed)
            });

            if matches!(hit, HitTest::Absorb) {
                return HitTest::Absorb;
            }
        }

        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;
        for index in 0..self.children.len() {
            needs |= self.children.get_mut(index).update_compositing_bits();
        }
        needs
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        for index in 0..self.children.len() {
            let child_offset = offset + self.offsets[index];
            self.children.get_mut(index).paint(ctx, child_offset);
        }
    }
}

/// Lays `children` out along the main axis: sizes the inflexible ones, shares the remaining space among the
/// flexible ones, resolves the flex's own size, and writes each child's `offset` and `size`. Returns the
/// flex's size.
// The loops address each child by index through `children`, so they cannot be plain iterators.
#[allow(clippy::needless_range_loop)]
fn flex_layout(
    children: &mut dyn FlexChildren,
    ctx: &mut LayoutCtx,
    config: FlexConfig,
    constraints: BoxConstraints,
    scope: LayoutScope,
    offsets: &mut Vec<Offset>,
    sizes: &mut Vec<Size>,
) -> Size {
    let main = config.direction;
    let cross = main.flip();
    let n = children.len();

    offsets.clear();
    offsets.resize(n, Offset::ZERO);
    sizes.clear();
    sizes.resize(n, Size::ZERO);

    let max_main = constraints.max_axis(main).get();
    let cross_max = constraints.max_axis(cross);
    let can_flex = max_main.is_finite();
    let stretch = matches!(config.cross_axis_alignment, CrossAxisAlignment::Stretch);

    let mut allocated = 0.0f32;
    let mut total_flex = 0.0f32;
    let mut cross_size = 0.0f32;

    // Pass 1: the inflexible children take their own size against an unbounded main axis. Every child also
    // gets this flex's scope deposited in its slot, so a later flex-factor change re-lays this flex.
    for index in 0..n {
        let data = flex_data_depositing(children.child(index).parent_data(), scope);
        if data.flex > 0.0 {
            total_flex += data.flex;
            continue;
        }
        let child = child_constraints(main, 0.0, f32::INFINITY, stretch, cross_max);
        let size = children.child(index).layout_and_get_size(ctx, child);
        sizes[index] = size;
        allocated += size.extent(main).get();
        cross_size = cross_size.max(size.extent(cross).get());
    }

    // Pass 2: the flexible children divide the leftover main-axis space by flex factor.
    let free = (max_main - allocated).max(0.0);
    let per_flex = if total_flex > 0.0 && can_flex {
        free / total_flex
    } else {
        0.0
    };
    for index in 0..n {
        let data = flex_data(children.child(index).parent_data());
        if data.flex <= 0.0 {
            continue;
        }
        let max_extent = per_flex * data.flex;
        let min_extent = if data.fit == FlexFit::Tight {
            max_extent
        } else {
            0.0
        };
        let child = child_constraints(main, min_extent, max_extent, stretch, cross_max);
        let size = children.child(index).layout_and_get_size(ctx, child);
        sizes[index] = size;
        allocated += size.extent(main).get();
        cross_size = cross_size.max(size.extent(cross).get());
    }

    let main_size = constraints
        .constrain_axis(
            main,
            own_main(config.main_axis_size, can_flex, max_main, allocated),
        )
        .get();
    let cross_final = own_cross(constraints, cross, stretch, cross_max, cross_size);

    // Position along the main axis, distributing the leftover space per the main-axis alignment.
    let free_main = (main_size - allocated).max(0.0);
    let (leading, between) = main_axis_spacing(config.main_axis_alignment, free_main, n);
    let flip = flip_main_axis(config);

    let baselines = match config.cross_axis_alignment {
        CrossAxisAlignment::Baseline(baseline) => Some(child_baselines(children, n, baseline)),
        _ => None,
    };
    let max_baseline = baselines
        .as_ref()
        .map(|b| b.iter().flatten().copied().fold(0.0f32, f32::max));

    let mut main_pos = leading;
    for index in 0..n {
        let child_main = sizes[index].extent(main).get();
        let child_cross = sizes[index].extent(cross).get();

        let cross_pos = match config.cross_axis_alignment {
            CrossAxisAlignment::Start | CrossAxisAlignment::Stretch => 0.0,
            CrossAxisAlignment::End => cross_final - child_cross,
            CrossAxisAlignment::Center => (cross_final - child_cross) / 2.0,
            CrossAxisAlignment::Baseline(_) => {
                match (max_baseline, baselines.as_ref().and_then(|b| b[index])) {
                    (Some(max), Some(child)) => max - child,
                    _ => 0.0,
                }
            }
        };

        let main_off = if flip {
            main_size - main_pos - child_main
        } else {
            main_pos
        };
        offsets[index] = offset_for(main, main_off, cross_pos);
        main_pos += child_main + between;
    }

    size_for(main, main_size, cross_final)
}

/// The flex factor and fit a child reported, or the inflexible default when it reported none.
fn flex_data(parent_data: &dyn Any) -> FlexData {
    parent_data
        .downcast_ref::<FlexParentData>()
        .map_or_else(FlexData::default, |flex| flex.data)
}

/// The child's flex data, also depositing `scope` into the child's slot so the child can mark this flex for
/// re-layout when its flex changes. The inflexible default when the child reported none.
fn flex_data_depositing(parent_data: &dyn Any, scope: LayoutScope) -> FlexData {
    match parent_data.downcast_ref::<FlexParentData>() {
        Some(flex) => {
            flex.container_scope.set(scope);
            flex.data
        }
        None => FlexData::default(),
    }
}

/// Each child's distance to `baseline`, in main-axis position order, [`None`] where a child has no baseline.
fn child_baselines(
    children: &mut dyn FlexChildren,
    n: usize,
    baseline: TextBaseline,
) -> Vec<Option<f32>> {
    (0..n)
        .map(|index| {
            children
                .child(index)
                .distance_to_baseline(baseline)
                .map(|distance| distance.get())
        })
        .collect()
}

/// The flex's own main-axis extent: the full incoming space when it expands and the space is bounded,
/// otherwise the space its children took.
fn own_main(main_axis_size: MainAxisSize, can_flex: bool, max_main: f32, allocated: f32) -> f32 {
    match main_axis_size {
        MainAxisSize::Max if can_flex => max_main,
        _ => allocated,
    }
}

/// The flex's own cross-axis extent: the full incoming space when children stretch into a bounded cross axis,
/// otherwise the widest child, clamped to the constraints.
fn own_cross(
    constraints: BoxConstraints,
    cross: Axis,
    stretch: bool,
    cross_max: Positive<f32>,
    cross_size: f32,
) -> f32 {
    let desired = if stretch && cross_max.get().is_finite() {
        cross_max.get()
    } else {
        cross_size
    };
    constraints.constrain_axis(cross, desired).get()
}

/// The child constraints for one flex slot: `main_min..main_max` along the main axis, and the cross axis
/// either tightened to `cross_max` when stretching into a bounded axis or loose up to it otherwise.
fn child_constraints(
    main: Axis,
    main_min: f32,
    main_max: f32,
    stretch: bool,
    cross_max: Positive<f32>,
) -> BoxConstraints {
    let (cross_min, cross_max) = if stretch && cross_max.get().is_finite() {
        (cross_max.get(), cross_max.get())
    } else {
        (0.0, cross_max.get())
    };

    match main {
        Axis::Horizontal => BoxConstraints::new(main_min, main_max, cross_min, cross_max),
        Axis::Vertical => BoxConstraints::new(cross_min, cross_max, main_min, main_max),
    }
}

/// The leading space before the first child and the gap between adjacent children, for `free` leftover
/// main-axis space across `n` children.
#[allow(clippy::cast_precision_loss)] // a child count never reaches f32's integer-precision limit
fn main_axis_spacing(alignment: MainAxisAlignment, free: f32, n: usize) -> (f32, f32) {
    let count = n as f32;
    match alignment {
        MainAxisAlignment::Start => (0.0, 0.0),
        MainAxisAlignment::End => (free, 0.0),
        MainAxisAlignment::Center => (free / 2.0, 0.0),
        MainAxisAlignment::SpaceBetween => (0.0, if n > 1 { free / (count - 1.0) } else { 0.0 }),
        MainAxisAlignment::SpaceAround => {
            let gap = if n > 0 { free / count } else { 0.0 };
            (gap / 2.0, gap)
        }
        MainAxisAlignment::SpaceEvenly => {
            let gap = free / (count + 1.0);
            (gap, gap)
        }
    }
}

/// Whether children run from the far end of the main axis back toward the start, per the vertical direction of
/// a column or the text direction of a row.
fn flip_main_axis(config: FlexConfig) -> bool {
    match config.direction {
        Axis::Vertical => config.vertical_direction == VerticalDirection::Up,
        Axis::Horizontal => config.text_direction == Some(TextDirection::RightToLeft),
    }
}

fn size_for(main: Axis, main_extent: f32, cross_extent: f32) -> Size {
    match main {
        Axis::Horizontal => Size::new(main_extent, cross_extent),
        Axis::Vertical => Size::new(cross_extent, main_extent),
    }
}

fn offset_for(main: Axis, main_pos: f32, cross_pos: f32) -> Offset {
    match main {
        Axis::Horizontal => Offset::new(main_pos, cross_pos),
        Axis::Vertical => Offset::new(cross_pos, main_pos),
    }
}
