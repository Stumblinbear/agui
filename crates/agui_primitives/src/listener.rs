use std::rc::Rc;

use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use agui_core::{
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
    type Element = SingleChildElement<Child::Element>;

    type Render = RenderPointerListener<Child::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let handler = self.handler();
        let behavior = self.behavior;
        let child = self.child;

        let (element, child_render) = SingleChildElement::new(child, ctx);

        (
            element,
            RenderPointerListener {
                handler,
                behavior,
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
        render_object.handler = self.handler();
        render_object.behavior = self.behavior;

        let child = self.child;

        element.update(child, &mut render_object.child.object, ctx);
    }
}

pub struct RenderPointerListener<Child> {
    handler: PointerHandler,
    behavior: HitTestBehavior,
    child: RenderNode<Child, Option<Size>>,
}

impl<Child> RenderObject for RenderPointerListener<Child>
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
}

impl<Child> RenderBox for RenderPointerListener<Child>
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
            result.add(self.handler.clone());
        }

        if hit { HitTest::Absorb } else { HitTest::Pass }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use agui_core::{
        input::pointer::PointerDispatcher,
        paint::compositing::{ContainerLayer, LayerHandle},
        pipeline::PipelineOwner,
        prelude::{element::*, render_object::*},
        test_harness::with_ctx,
    };

    use crate::{padding::Padding, sized_box::SizedBox};

    use super::*;

    /// End to end: hit-test the root, then dispatch a pointer down and confirm the listener's handler
    /// runs with the position localized into its own space.
    #[test]
    fn a_down_reaches_the_listener_localized() {
        let local = Rc::new(Cell::new(None));

        let on_down: PointerHandler = {
            let local = Rc::clone(&local);
            Rc::new(move |event: &PointerEvent| local.set(Some(event.position)))
        };

        let widget = Padding::new(EdgeInsets::all(10.0)).child(
            Listener::builder()
                .on_pointer_down(on_down)
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );

        let (_, render) = with_ctx(|ctx| widget.create(ctx));

        let mut owner = PipelineOwner::new(
            Rc::new(RefCell::new(render)),
            LayerHandle::new(ContainerLayer::new()),
        );
        owner.resize(BoxConstraints::new(0, 100, 0, 100));
        owner.flush_layout();

        let mut dispatcher = PointerDispatcher::new();
        dispatcher.handle(
            &PointerEvent {
                pointer: PointerId(1),
                position: Offset::new(35.0, 40.0),
                kind: PointerEventKind::Down,
            },
            |position| owner.hit_test(position),
        );

        let got = local.get().expect("the listener handled the down");
        assert_eq!(got.x.get(), 25.0);
        assert_eq!(got.y.get(), 30.0);
    }
}

#[cfg(test)]
mod harness {
    use std::{cell::Cell, rc::Rc};

    use agui_test::prelude::*;

    use super::Listener;
    use crate::sized_box::SizedBox;

    #[test]
    fn a_tap_reaches_a_listener_under_the_pointer() {
        let taps = Rc::new(Cell::new(0));

        let handler: PointerHandler = {
            let taps = Rc::clone(&taps);
            Rc::new(move |_: &PointerEvent| taps.set(taps.get() + 1))
        };

        let mut tester = WidgetTester::mount(
            Listener::builder()
                .on_pointer_down(handler)
                .behavior(HitTestBehavior::Opaque)
                .child(TestBox::new(Size::new(50, 50))),
        );

        tester.resize(Size::new(50, 50));
        tester.pump(Duration::ZERO);

        tester.tap_at(Offset::new(25, 25));

        assert_eq!(taps.get(), 1);
    }

    #[test]
    fn obeys_the_box_sizing_contracts() {
        BoxSizingCheck::default()
            .run(|| Listener::builder().child(SizedBox::new().width(20).height(10)));
    }
}
