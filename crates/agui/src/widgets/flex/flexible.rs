use std::any::Any;

use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

use super::{FlexData, FlexFit, FlexParentData};

/// A child of a [`Row`](super::Row) or [`Column`](super::Column) that flexes to fill the free main-axis
/// space, sharing it with its flex siblings in proportion to its flex factor. A child not wrapped in one of
/// these is inflexible, sized to its own content.
#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Flexible<Child> {
    #[builder(finish_fn)]
    child: Child,

    /// This child's share of the free main-axis space, relative to its flex siblings.
    #[builder(default = 1.0)]
    flex: f32,

    /// Whether the child fills the space its flex factor wins, or may be smaller.
    #[builder(default = FlexFit::Loose)]
    fit: FlexFit,
}

/// A child of a [`Row`](super::Row) or [`Column`](super::Column) that fills the free main-axis space, sharing
/// it with its flex siblings in proportion to its flex factor. The same as a [`Flexible`] with a tight fit.
#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Expanded<Child> {
    #[builder(finish_fn)]
    child: Child,

    /// This child's share of the free main-axis space, relative to its flex siblings.
    #[builder(default = 1.0)]
    flex: f32,
}

impl<Child> Widget for Flexible<Child>
where
    Child: Widget,
    Child::Render: RenderBox + Sized,
{
    type Element = SingleChildElement<Child::Element, RenderFlexible<Child::Render>>;

    type Render = RenderFlexible<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderFlexible::new(FlexData {
                flex: self.flex,
                fit: self.fit,
            }),
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.render_object_mut().set_flex_data(
            ctx,
            FlexData {
                flex: self.flex,
                fit: self.fit,
            },
        );
        element.update(ctx, self.child);
    }
}

impl<Child> Widget for Expanded<Child>
where
    Child: Widget,
    Child::Render: RenderBox + Sized,
{
    type Element = SingleChildElement<Child::Element, RenderFlexible<Child::Render>>;

    type Render = RenderFlexible<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderFlexible::new(FlexData {
                flex: self.flex,
                fit: FlexFit::Tight,
            }),
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.render_object_mut().set_flex_data(
            ctx,
            FlexData {
                flex: self.flex,
                fit: FlexFit::Tight,
            },
        );
        element.update(ctx, self.child);
    }
}

/// The render object of a [`Flexible`] or [`Expanded`]: a transparent box that lays its child out verbatim and
/// reports its flex parent data for the enclosing flex to read.
pub struct RenderFlexible<Child: ?Sized> {
    flex_parent_data: FlexParentData,

    child: RenderNode<Child>,
}

impl<Child: ?Sized> RenderFlexible<Child> {
    fn new(flex_data: FlexData) -> Self {
        Self {
            flex_parent_data: FlexParentData::new(flex_data),
            child: RenderNode::new(()),
        }
    }

    /// Replaces the flex data and re-lays the enclosing flex when it changes, since the new factor reshapes
    /// how every flex sibling shares the main-axis space.
    fn set_flex_data(&mut self, ctx: &mut UpdateCtx, flex_data: FlexData) {
        if self.flex_parent_data.data != flex_data {
            self.flex_parent_data.data = flex_data;

            // The flex deposits its own relayout scope here each layout. Marking it re-runs the flex so it
            // re-divides the space. Before the flex has laid out once the scope is detached, which marks as a
            // no-op, and that first layout makes the split.
            ctx.mark_needs_layout(self.flex_parent_data.container_scope.get());
        }
    }
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderFlexible<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderFlexible<Child> {
    fn detach(&mut self, _ctx: &mut UpdateCtx<'_>) {
        // Forget the flex's deposited scope. While detached, a flex change must not mark a flex this child has
        // left. Re-attaching and laying out deposits the right scope again.
        self.flex_parent_data
            .container_scope
            .set(LayoutScope::detached());
    }

    fn parent_data(&self) -> &dyn Any {
        &self.flex_parent_data
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.describe(d)
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderFlexible<Child> {
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

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }
}
