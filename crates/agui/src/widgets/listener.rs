use std::rc::Rc;

use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::{
    input::pointer::PointerHandler,
    prelude::{element::*, render_object::*},
};

/// A widget that receives pointer events landing on its child, without affecting layout or painting.
///
/// `behavior` decides whether the listener counts as hit on its own: by default it is hit only where
/// its child is, but it can claim its whole area and either block or pass through what is behind it.
#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Listener<Child> {
    #[builder(finish_fn)]
    child: Child,

    on_pointer_down: Option<PointerHandler>,
    on_pointer_move: Option<PointerHandler>,
    on_pointer_up: Option<PointerHandler>,
    on_pointer_cancel: Option<PointerHandler>,

    #[builder(default)]
    behavior: HitTestBehavior,
}

impl<Child> Listener<Child> {
    /// Folds the per-kind callbacks into a single handler that routes each event to the matching one.
    fn handler(&self) -> PointerHandler {
        let down = self.on_pointer_down.clone();
        let moved = self.on_pointer_move.clone();
        let up = self.on_pointer_up.clone();
        let cancel = self.on_pointer_cancel.clone();

        Rc::new(move |event: &PointerEvent| {
            let handler = match event.kind {
                PointerEventKind::Down => &down,
                PointerEventKind::Move => &moved,
                PointerEventKind::Up => &up,
                PointerEventKind::Cancel => &cancel,
            };

            if let Some(handler) = handler {
                handler(event);
            }
        })
    }
}

impl<Child> Widget for Listener<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderPointerListener<Child::Render>>;

    type Render = RenderPointerListener<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let handler = self.handler();

        SingleChildElement::new(
            ctx,
            self.child,
            RenderPointerListener {
                handler,
                behavior: self.behavior,
                child: RenderNode::new(None),
            },
        )
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let render = element.render_object_mut();
        render.handler = self.handler();
        render.behavior = self.behavior;

        element.update(ctx, self.child);
    }
}

pub struct RenderPointerListener<Child: ?Sized> {
    handler: PointerHandler,
    behavior: HitTestBehavior,
    child: RenderNode<Child, Option<Size>>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderPointerListener<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderPointerListener<Child> {
    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        self.child.build_semantics(s);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("behavior", self.behavior)
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderPointerListener<Child> {
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
        let size = self.child.layout_and_get_size(ctx, constraints);
        self.child.parent_data = Some(size);
        size
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
        let Some(size) = self.child.parent_data else {
            return HitTest::Pass;
        };

        if !size.contains(position) {
            return HitTest::Pass;
        }

        let child_hit = self.child.hit_test(result, position) == HitTest::Absorb;
        let hit = child_hit || self.behavior == HitTestBehavior::Opaque;

        // Children record themselves first; a translucent listener records even when none were hit.
        if hit || self.behavior == HitTestBehavior::Translucent {
            result.add(Rc::clone(&self.handler));
        }

        if hit { HitTest::Absorb } else { HitTest::Pass }
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}
