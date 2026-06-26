use typed_floats::{Positive, PositiveFinite};

use crate::prelude::{element::*, render_object::*};

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

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        SingleChildElement::new(
            ctx,
            self.child,
            RenderCenter {
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        element.update(ctx, self.child);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ChildParentData {
    size: Size,
    offset: Offset,
}

pub struct RenderCenter<Child: ?Sized> {
    child: RenderNode<Child, Option<ChildParentData>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderCenter<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: ?Sized> RenderCenter<Child> {
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

impl<Child: RenderBox + ?Sized> RenderObject for RenderCenter<Child> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        let offset = self
            .child
            .parent_data
            .as_ref()
            .map_or(Offset::ZERO, |data| data.offset);

        s.with_offset(offset, |s| self.child.build_semantics(s));
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderCenter<Child> {
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

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
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
