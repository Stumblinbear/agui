use std::any::Any;
use std::cell::Cell;

use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::geometry::Alignment;
use crate::prelude::{element::*, render_object::*};
use crate::widget::{ChildrenElement, WidgetSequence};

/// How a [`Stack`] sizes the children that are not [`Positioned`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StackFit {
    /// Each child may be any size up to the stack, under loosened constraints.
    #[default]
    Loose,

    /// Each child is forced to fill the stack, under tight constraints.
    Expand,

    /// Each child is given the stack's own incoming constraints unchanged.
    Passthrough,
}

/// The edges and size a [`Positioned`] pins its child to within the [`Stack`]. An unset edge or extent leaves
/// that part of the child's placement to the stack's alignment.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PositionedData {
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// The parent data a positioned child reports up to its [`Stack`]: its [`PositionedData`], plus a slot the
/// stack writes its own relayout scope into during layout, which the child reads back to re-lay the stack
/// when its position changes.
struct StackParentData {
    data: PositionedData,
    container_scope: Cell<LayoutScope>,
}

impl StackParentData {
    fn new(data: PositionedData) -> Self {
        Self {
            data,
            container_scope: Cell::new(LayoutScope::detached()),
        }
    }
}

/// A child of a [`Stack`] pinned to its given edges, sized by the edges it sets. A child not wrapped in one of
/// these is non-positioned: sized by the stack's [`StackFit`] and placed by its alignment.
#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Positioned<Child> {
    #[builder(finish_fn)]
    child: Child,

    /// Distance from the stack's top edge to the child's top edge.
    top: Option<f32>,

    /// Distance from the stack's right edge to the child's right edge.
    right: Option<f32>,

    /// Distance from the stack's bottom edge to the child's bottom edge.
    bottom: Option<f32>,

    /// Distance from the stack's left edge to the child's left edge.
    left: Option<f32>,

    /// The child's width. Ignored when both `left` and `right` are set, which pin the width instead.
    width: Option<f32>,

    /// The child's height. Ignored when both `top` and `bottom` are set, which pin the height instead.
    height: Option<f32>,
}

impl<Child> Positioned<Child> {
    fn positioned_data(&self) -> PositionedData {
        PositionedData {
            top: self.top,
            right: self.right,
            bottom: self.bottom,
            left: self.left,
            width: self.width,
            height: self.height,
        }
    }
}

impl<Child> Widget for Positioned<Child>
where
    Child: Widget,
    Child::Render: RenderBox + Sized,
{
    type Element = SingleChildElement<Child::Element, RenderPositioned<Child::Render>>;

    type Render = RenderPositioned<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let data = self.positioned_data();
        SingleChildElement::new(ctx, self.child, RenderPositioned::new(data))
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let data = self.positioned_data();
        element.render_object_mut().set_positioned_data(ctx, data);
        element.update(ctx, self.child);
    }
}

/// The render object of a [`Positioned`]: a transparent box that lays its child out verbatim and reports its
/// positioned parent data for the enclosing stack to read.
pub struct RenderPositioned<Child: ?Sized> {
    stack_parent_data: StackParentData,

    child: RenderNode<Child>,
}

impl<Child: ?Sized> RenderPositioned<Child> {
    fn new(data: PositionedData) -> Self {
        Self {
            stack_parent_data: StackParentData::new(data),
            child: RenderNode::new(()),
        }
    }

    /// Replaces the positioned data and re-lays the enclosing stack when it changes, since the new edges move
    /// where the stack places the child.
    fn set_positioned_data(&mut self, ctx: &mut UpdateCtx, data: PositionedData) {
        if self.stack_parent_data.data != data {
            self.stack_parent_data.data = data;

            // The stack deposits its own relayout scope here each layout. Marking it re-runs the stack so it
            // re-places the child. Before the stack has laid out once the scope is detached, which marks as a
            // no-op, and that first layout does the placement.
            ctx.mark_needs_layout(self.stack_parent_data.container_scope.get());
        }
    }
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderPositioned<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderPositioned<Child> {
    fn detach(&mut self, _ctx: &mut UpdateCtx<'_>) {
        // Forget the stack's deposited scope. While detached, a position change must not mark a stack this
        // child has left. Re-attaching and laying out deposits the right scope again.
        self.stack_parent_data
            .container_scope
            .set(LayoutScope::detached());
    }

    fn parent_data(&self) -> &dyn Any {
        &self.stack_parent_data
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.describe(d)
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderPositioned<Child> {
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

/// A widget that layers its children on top of one another, the later children drawn over the earlier ones.
///
/// A child wrapped in [`Positioned`] is pinned to the edges it sets. A non-positioned child is sized by the
/// fit and placed by the alignment. The stack sizes itself to its non-positioned children, or fills its
/// constraints when it has none.
#[derive(Builder)]
pub struct Stack<Children> {
    #[builder(default = Alignment::TOP_LEFT)]
    alignment: Alignment,

    #[builder(default)]
    fit: StackFit,

    children: Children,
}

impl<Children> Stack<Children> {
    fn config(&self) -> StackConfig {
        StackConfig {
            alignment: self.alignment,
            fit: self.fit,
        }
    }
}

impl<Children> Widget for Stack<Children>
where
    Children: WidgetSequence,
    Children::Renders: RenderChildren<ChildData = ()> + 'static,
{
    type Element = ChildrenElement<Children, RenderStack<Children::Renders>>;

    type Render = RenderStack<Children::Renders>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let config = self.config();
        ChildrenElement::new(ctx, self.children, |children| {
            RenderStack::new(config, children)
        })
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let config = self.config();
        element.render_object_mut().set_config(ctx, config);
        element.update(ctx, self.children);
    }
}

/// The fixed configuration of a stack, set by the [`Stack`] that builds it and read each layout.
#[derive(Clone, Copy, PartialEq)]
pub struct StackConfig {
    pub alignment: Alignment,
    pub fit: StackFit,
}

/// A flat, mutable view of a stack's child render nodes, addressed by index.
// The layout core runs over this trait object so it compiles once, not once per concrete child-list shape.
trait StackChildren {
    fn len(&self) -> usize;
    fn child(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox>;
}

impl<C: RenderChildren<ChildData = ()>> StackChildren for C {
    fn len(&self) -> usize {
        RenderChildren::len(self)
    }

    fn child(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox> {
        self.get_mut(index)
    }
}

/// The render object of a [`Stack`]: layers its children, sizing itself to the non-positioned ones and pinning
/// the positioned ones to their edges.
pub struct RenderStack<Children> {
    config: StackConfig,

    layout_scope: LayoutScope,
    children: Children,

    offsets: Vec<Offset>,
    child_sizes: Vec<Size>,

    size: Size,
}

impl<Children> RenderStack<Children> {
    pub fn new(config: StackConfig, children: Children) -> Self {
        Self {
            config,
            layout_scope: LayoutScope::detached(),
            children,
            offsets: Vec::new(),
            child_sizes: Vec::new(),
            size: Size::ZERO,
        }
    }

    /// Replaces the configuration and re-lays the stack when it changes.
    pub fn set_config(&mut self, ctx: &mut UpdateCtx, config: StackConfig) {
        if self.config != config {
            self.config = config;
            ctx.mark_needs_layout(self.layout_scope);
        }
    }
}

impl<Children: RenderChildren<ChildData = ()> + 'static> MultiChildRenderObject
    for RenderStack<Children>
{
    type Children = Children;

    fn children_mut(&mut self) -> &mut Children {
        &mut self.children
    }

    fn layout_scope(&self) -> LayoutScope {
        self.layout_scope
    }
}

impl<Children: RenderChildren<ChildData = ()> + 'static> RenderObject for RenderStack<Children> {
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

impl<Children: RenderChildren<ChildData = ()> + 'static> RenderBox for RenderStack<Children> {
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
        let non_positioned = non_positioned_constraints(constraints, self.config.fit);

        let mut width = constraints.min_width().get();
        let mut height = constraints.min_height().get();
        let mut has_non_positioned = false;

        for index in 0..self.children.len() {
            if positioned_data(self.children.get(index).parent_data()).is_some() {
                continue;
            }
            has_non_positioned = true;
            let size = self.children.get(index).measure(non_positioned);
            width = width.max(size.width.get());
            height = height.max(size.height.get());
        }

        stack_size(constraints, has_non_positioned, width, height)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = *ctx.scope();

        let size = stack_layout(
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

/// Lays `children` out in a stack: sizes the non-positioned ones and aligns them, pins the positioned ones to
/// their edges, and writes each child's `offset` and `size`. Returns the stack's size.
#[allow(clippy::needless_range_loop)]
fn stack_layout(
    children: &mut dyn StackChildren,
    ctx: &mut LayoutCtx,
    config: StackConfig,
    constraints: BoxConstraints,
    scope: LayoutScope,
    offsets: &mut Vec<Offset>,
    sizes: &mut Vec<Size>,
) -> Size {
    let n = children.len();

    offsets.clear();
    offsets.resize(n, Offset::ZERO);
    sizes.clear();
    sizes.resize(n, Size::ZERO);

    let non_positioned = non_positioned_constraints(constraints, config.fit);

    // First, lay out the non-positioned children. They set the stack's size, and the positioned ones are
    // placed within it afterward. Every child also gets this stack's scope deposited so a later position
    // change re-lays this stack.
    let mut width = constraints.min_width().get();
    let mut height = constraints.min_height().get();
    let mut has_non_positioned = false;

    for index in 0..n {
        if positioned_data_depositing(children.child(index).parent_data(), scope).is_some() {
            continue;
        }
        has_non_positioned = true;
        let size = children
            .child(index)
            .layout_and_get_size(ctx, non_positioned);
        sizes[index] = size;
        width = width.max(size.width.get());
        height = height.max(size.height.get());
    }

    let size = stack_size(constraints, has_non_positioned, width, height);

    // Place every child: align the non-positioned ones, pin and lay out the positioned ones.
    for index in 0..n {
        match positioned_data(children.child(index).parent_data()) {
            None => {
                let child = sizes[index];
                offsets[index] = config.alignment.along_offset(Offset::new(
                    size.width.get() - child.width.get(),
                    size.height.get() - child.height.get(),
                ));
            }
            Some(data) => {
                let child = children
                    .child(index)
                    .layout_and_get_size(ctx, positioned_constraints(data, size));
                sizes[index] = child;
                offsets[index] = positioned_offset(data, config.alignment, size, child);
            }
        }
    }

    size
}

/// The constraints a non-positioned child is laid out under, per the stack's fit.
fn non_positioned_constraints(constraints: BoxConstraints, fit: StackFit) -> BoxConstraints {
    match fit {
        StackFit::Loose => constraints.loosen(),
        StackFit::Expand => BoxConstraints::tight(constraints.biggest()),
        StackFit::Passthrough => constraints,
    }
}

/// The stack's own size: the largest non-positioned child clamped to the constraints, or the constraints'
/// biggest when there are none and that is bounded, otherwise their smallest.
fn stack_size(
    constraints: BoxConstraints,
    has_non_positioned: bool,
    width: f32,
    height: f32,
) -> Size {
    if has_non_positioned {
        return constraints.constrain(Size::new(width, height));
    }

    let biggest = constraints.biggest();
    if biggest.is_finite() {
        biggest
    } else {
        constraints.smallest()
    }
}

/// The constraints a positioned child is laid out under: a tight extent on each axis whose edges or size pin
/// it, otherwise free up to unbounded.
fn positioned_constraints(data: PositionedData, size: Size) -> BoxConstraints {
    let (min_width, max_width) = match (data.left, data.right, data.width) {
        (Some(left), Some(right), _) => {
            let width = (size.width.get() - left - right).max(0.0);
            (width, width)
        }
        (_, _, Some(width)) => {
            let width = width.max(0.0);
            (width, width)
        }
        _ => (0.0, f32::INFINITY),
    };

    let (min_height, max_height) = match (data.top, data.bottom, data.height) {
        (Some(top), Some(bottom), _) => {
            let height = (size.height.get() - top - bottom).max(0.0);
            (height, height)
        }
        (_, _, Some(height)) => {
            let height = height.max(0.0);
            (height, height)
        }
        _ => (0.0, f32::INFINITY),
    };

    BoxConstraints::new(min_width, max_width, min_height, max_height)
}

/// Where a positioned child lands: pinned to whichever of its edges is set, falling back to the stack's
/// alignment on an axis it leaves free.
fn positioned_offset(
    data: PositionedData,
    alignment: Alignment,
    size: Size,
    child: Size,
) -> Offset {
    let free = Offset::new(
        size.width.get() - child.width.get(),
        size.height.get() - child.height.get(),
    );
    let aligned = alignment.along_offset(free);

    let x = if let Some(left) = data.left {
        left
    } else if let Some(right) = data.right {
        size.width.get() - right - child.width.get()
    } else {
        aligned.x.get()
    };

    let y = if let Some(top) = data.top {
        top
    } else if let Some(bottom) = data.bottom {
        size.height.get() - bottom - child.height.get()
    } else {
        aligned.y.get()
    };

    Offset::new(x, y)
}

/// The positioned data a child reported, or [`None`] when it is non-positioned.
fn positioned_data(parent_data: &dyn Any) -> Option<PositionedData> {
    parent_data
        .downcast_ref::<StackParentData>()
        .map(|positioned| positioned.data)
}

/// The positioned data a child reported, also depositing `scope` into the child's slot so the child can mark
/// this stack for re-layout when its position changes. [`None`] when the child is non-positioned.
fn positioned_data_depositing(parent_data: &dyn Any, scope: LayoutScope) -> Option<PositionedData> {
    parent_data
        .downcast_ref::<StackParentData>()
        .map(|positioned| {
            positioned.container_scope.set(scope);
            positioned.data
        })
}
