use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

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

    /// Sizes the child to `factor` of the available width.
    ///
    /// # Panics
    /// Panics if `factor` is not a non-negative, finite number.
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

    /// Sizes the child to `factor` of the available height.
    ///
    /// # Panics
    /// Panics if `factor` is not a non-negative, finite number.
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

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderFractionallySizedBox {
                width_factor: self.width_factor,
                height_factor: self.height_factor,
                alignment: self.alignment,

                layout_scope: LayoutScope::detached(),
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        if render.width_factor != self.width_factor
            || render.height_factor != self.height_factor
            || render.alignment != self.alignment
        {
            render.width_factor = self.width_factor;
            render.height_factor = self.height_factor;
            render.alignment = self.alignment;

            ctx.mark_needs_layout(render.layout_scope);
        }

        element.update(ctx, self.child);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderFractionallySizedBox<Child: ?Sized> {
    width_factor: Option<PositiveFinite<f32>>,
    height_factor: Option<PositiveFinite<f32>>,
    alignment: Alignment,

    layout_scope: LayoutScope,
    child: RenderNode<Child, Option<ChildParentData>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderFractionallySizedBox<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: ?Sized> RenderFractionallySizedBox<Child> {
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

impl<Child: RenderBox + ?Sized> RenderObject for RenderFractionallySizedBox<Child> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property_opt("width_factor", self.width_factor)
            .property_opt("height_factor", self.height_factor)
            .property("alignment", self.alignment)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderFractionallySizedBox<Child> {
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
        self.layout_scope = *ctx.scope();

        let child_size = self
            .child
            .layout_and_get_size(ctx, self.inner_constraints(constraints));
        let size = constraints.constrain(child_size);

        let offset = self.alignment.along_offset(Offset::new(
            size.width.get() - child_size.width.get(),
            size.height.get() - child_size.height.get(),
        ));

        self.child.child_data = Some(ChildParentData { size, offset });

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
            self.child.child_data.expect("child has not been laid out");

        if !size.contains(position) {
            return HitTest::Pass;
        }

        result.with_offset(offset, position, |result, transformed| {
            self.child.hit_test(result, transformed)
        })
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let child_offset = self
            .child
            .child_data
            .expect("child has not been laid out")
            .offset;

        self.child.paint(ctx, offset + child_offset);
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        let offset = self
            .child
            .child_data
            .as_ref()
            .map_or(Offset::ZERO, |data| data.offset);

        s.with_offset(offset, |s| self.child.build_semantics(s));
    }
}
