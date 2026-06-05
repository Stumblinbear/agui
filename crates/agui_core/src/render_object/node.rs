use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    paint::PaintCtx,
    render_object::{LayoutCtx, MountCtx, RenderObject, box_layout::RenderBox},
    size::Size,
    text_baseline::TextBaseline,
};

pub struct RenderNode<R, P = ()> {
    pub object: R,
    pub parent_data: P,

    parent_uses_size: bool,
    needs_compositing: bool,
}

impl<R, P: Default> RenderNode<R, P> {
    pub fn new(object: R) -> Self {
        Self {
            object,
            parent_data: P::default(),

            parent_uses_size: false,
            needs_compositing: false,
        }
    }
}

impl<R, P> RenderNode<R, P> {
    pub fn new_with(object: R, parent_data: P) -> Self {
        Self {
            object,
            parent_data,

            parent_uses_size: false,
            needs_compositing: false,
        }
    }
}

impl<R: RenderObject, P> RenderNode<R, P> {
    pub fn mount(&mut self, ctx: &mut MountCtx) {
        self.object.mount(ctx);
    }

    pub fn unmount(&mut self, ctx: &mut MountCtx) {
        self.object.unmount(ctx);
    }

    pub fn update_compositing_bits(&mut self) -> bool {
        self.needs_compositing = self.object.update_compositing_bits();
        self.needs_compositing
    }

    /// Whether this child's subtree contributes a compositing layer, as of the last recompute.
    pub fn needs_compositing(&self) -> bool {
        self.needs_compositing
    }
}

impl<R: RenderBox, P> RenderNode<R, P> {
    pub fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.object.min_intrinsic_width(height)
    }

    pub fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.object.max_intrinsic_width(height)
    }

    pub fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.object.min_intrinsic_height(width)
    }

    pub fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.object.max_intrinsic_height(width)
    }

    pub fn measure(&self, constraints: Constraints) -> Size {
        self.object.measure(constraints)
    }

    /// Lay this child out under `constraints`. If you need the resulting size of the child, use `layout_and_get_size` instead.
    pub fn layout(&mut self, ctx: &mut LayoutCtx, constraints: Constraints) {
        self.object.layout(ctx, constraints);
    }

    /// Lay this child out under `constraints` and return the size it took. This couples the child with the parent so that when
    /// the child's layout changes, the parent is also laid out.
    pub fn layout_and_get_size(&mut self, ctx: &mut LayoutCtx, constraints: Constraints) -> Size {
        let size = self.object.layout(ctx, constraints);
        self.parent_uses_size = true;
        size
    }

    pub fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.object.measure_baseline(constraints, baseline)
    }

    pub fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.object.distance_to_baseline(baseline)
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.object.hit_test(result, position)
    }

    pub fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.object.paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::*;
    use crate::{
        context::UpdateCtx,
        element::{Element, ElementNode},
        paint::{ContainerLayer, LayerHandle},
        render_object::box_layout::{AnyRenderBox, RenderBox},
        test_fixtures::Leaf,
        test_harness::TestHarness,
        text_baseline::TextBaseline,
        widget::{AsAnyWidget, Widget},
    };

    struct RenderPad<C: RenderBox> {
        pad: u32,
        child: RenderNode<C>,
    }

    impl<C: RenderBox> RenderObject for RenderPad<C> {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            self.child.update_compositing_bits()
        }
    }

    impl<C: RenderBox> RenderBox for RenderPad<C> {
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

        fn measure(&self, constraints: Constraints) -> Size {
            self.child.measure(constraints)
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: Constraints) -> Size {
            self.child.layout_and_get_size(ctx, constraints)
        }

        fn measure_baseline(
            &self,
            constraints: Constraints,
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

    struct Pad<Child> {
        pad: u32,
        child: Child,
    }

    struct PadElement<C> {
        child: ElementNode<C>,
    }

    impl<C: Element> Element for PadElement<C> {}

    impl<Child: Widget> Widget for Pad<Child>
    where
        Child::Render: RenderBox,
    {
        type Element = PadElement<Child::Element>;

        type Render = RenderPad<Child::Render>;

        fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
            PadElement {
                child: ElementNode::new(self.child.create_element(ctx)),
            }
        }

        fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
            self.child
                .update(&mut element.child.element, &old.child, ctx);
        }

        fn create_render_object(&self, element: &Self::Element) -> Self::Render {
            RenderPad {
                pad: self.pad,
                child: RenderNode::new(self.child.create_render_object(&element.child.element)),
            }
        }

        fn update_render_object(&self, element: &Self::Element, object: &mut Self::Render) {
            object.pad = self.pad;

            self.child
                .update_render_object(&element.child.element, &mut object.child.object);
        }
    }

    #[test]
    fn create_render_object_builds_wrapped_subtree() {
        let widget = Pad {
            pad: 4,
            child: Leaf::new(),
        };
        let harness = TestHarness::mount(&widget);

        let node = RenderNode::<_, ()>::new(widget.create_render_object(&harness.root.element));

        assert_eq!(node.object.pad, 4);
    }

    #[test]
    fn update_render_object_syncs_in_place() {
        let widget = Pad {
            pad: 4,
            child: Leaf::new(),
        };

        let harness = TestHarness::mount(&widget);
        let mut node = RenderNode::<_, ()>::new(widget.create_render_object(&harness.root.element));

        assert_eq!(node.object.pad, 4);

        Pad {
            pad: 9,
            child: Leaf::new(),
        }
        .update_render_object(&harness.root.element, &mut node.object);

        assert_eq!(node.object.pad, 9);
    }

    #[test]
    fn node_layout_and_paint_helpers() {
        let mut node = RenderNode::<(), ()>::new(());

        let size = node.layout_and_get_size(
            &mut LayoutCtx::detached(),
            Constraints::tight(Size::new(10.0, 20.0)),
        );
        assert_eq!(size, Size::new(10.0, 20.0));

        PaintCtx::paint(&LayerHandle::new(ContainerLayer::new()), |ctx| {
            node.paint(ctx, Offset::ZERO);
        });
    }

    #[test]
    fn build_update_paint_layout_and_reconcile_a_boxed_child() {
        // construct a render tree through the real Widget seam
        let widget = Pad {
            pad: 4,
            child: Leaf::new(),
        };
        let harness = TestHarness::mount(&widget);
        let mut node = RenderNode::<_, ()>::new(widget.create_render_object(&harness.root.element));
        assert_eq!(node.object.pad, 4);

        // update it in place
        Pad {
            pad: 9,
            child: Leaf::new(),
        }
        .update_render_object(&harness.root.element, &mut node.object);
        assert_eq!(node.object.pad, 9);

        // paint the whole tree; lay out the (RenderBox) leaf child via the node helpers
        PaintCtx::paint(&LayerHandle::new(ContainerLayer::new()), |ctx| {
            node.paint(ctx, Offset::ZERO);
        });
        assert_eq!(
            node.object.child.layout_and_get_size(
                &mut LayoutCtx::detached(),
                Constraints::tight(Size::new(12.0, 8.0))
            ),
            Size::new(12.0, 8.0),
        );

        // erased boundary: a boxed child, reconciled by recovering its real type —
        // this is what a fan-out's update_render_object does per slot.
        let mut boxed: RenderNode<Box<dyn AnyRenderBox>, ()> =
            RenderNode::new(Box::new(()) as Box<dyn AnyRenderBox>);

        // a wrong type does not match...
        assert!(
            (*boxed.object)
                .as_any_mut()
                .downcast_mut::<RenderOther>()
                .is_none()
        );

        // ...the correct type does, and we update it in place through the recovered object
        (*boxed.object)
            .as_any_mut()
            .downcast_mut::<()>()
            .expect("boxed child downcasts to its real type");

        assert_eq!(
            boxed.layout_and_get_size(
                &mut LayoutCtx::detached(),
                Constraints::tight(Size::new(7.0, 7.0))
            ),
            Size::new(7.0, 7.0)
        );

        // the boxed node still drives layout/paint through dyn dispatch
        assert_eq!(
            boxed.layout_and_get_size(
                &mut LayoutCtx::detached(),
                Constraints::tight(Size::new(3.0, 3.0))
            ),
            Size::new(3.0, 3.0),
        );

        PaintCtx::paint(&LayerHandle::new(ContainerLayer::new()), |ctx| {
            boxed.paint(ctx, Offset::ZERO);
        });
    }

    struct Counted {
        creates: Rc<Cell<usize>>,
    }

    struct CountedElement;

    impl Element for CountedElement {}

    impl Widget for Counted {
        type Element = CountedElement;

        type Render = ();

        fn create_element(&self, _: &mut UpdateCtx) -> CountedElement {
            CountedElement
        }

        fn update(&self, _: &mut CountedElement, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &CountedElement) -> Self::Render {
            self.creates.set(self.creates.get() + 1);
        }

        fn update_render_object(&self, _: &CountedElement, (): &mut Self::Render) {}
    }

    #[test]
    fn updating_a_boxed_slot_reuses_the_render_object() {
        let creates = Rc::new(Cell::new(0usize));

        let widget = Counted {
            creates: Rc::clone(&creates),
        }
        .into_boxed_render_box();

        let harness = TestHarness::mount(&widget);

        // a fan-out slot: a boxed child wrapped in a RenderNode
        let mut slot: RenderNode<Box<dyn AnyRenderBox>, ()> =
            RenderNode::new(widget.create_render_object(&harness.root.element));

        assert_eq!(creates.get(), 1);

        // same concrete type -> reuse the boxed render object, do not recreate
        widget.update_render_object(&harness.root.element, &mut slot.object);

        assert_eq!(
            creates.get(),
            1,
            "same-type update must reuse the boxed render object, not recreate it"
        );
    }

    #[derive(Default)]
    struct RenderOther {}

    impl RenderObject for RenderOther {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for RenderOther {
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

        fn measure(&self, constraints: Constraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, _: &mut LayoutCtx, constraints: Constraints) -> Size {
            constraints.smallest()
        }

        fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
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

    struct CountedOther {
        creates: Rc<Cell<usize>>,
    }

    struct CountedOtherElement;

    impl Element for CountedOtherElement {}

    impl Widget for CountedOther {
        type Element = CountedOtherElement;

        type Render = RenderOther;

        fn create_element(&self, _: &mut UpdateCtx) -> CountedOtherElement {
            CountedOtherElement
        }

        fn update(&self, _: &mut CountedOtherElement, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &CountedOtherElement) -> Self::Render {
            self.creates.set(self.creates.get() + 1);

            RenderOther::default()
        }

        fn update_render_object(&self, _: &CountedOtherElement, _: &mut Self::Render) {}
    }

    #[test]
    fn type_swap_at_a_boxed_slot_recreates_the_render_object() {
        let creates_a = Rc::new(Cell::new(0usize));
        let widget_a = Counted {
            creates: Rc::clone(&creates_a),
        }
        .into_boxed_render_box();
        let mut harness = TestHarness::mount(&widget_a);

        let mut slot: RenderNode<Box<dyn AnyRenderBox>, ()> =
            RenderNode::new(widget_a.create_render_object(&harness.root.element));
        assert_eq!(creates_a.get(), 1);
        assert!((*slot.object).as_any_mut().downcast_mut::<()>().is_some());

        // swap to a different concrete render type -> reconcile the element, then the slot's
        // render object downcast fails -> recreate
        let creates_b = Rc::new(Cell::new(0usize));
        let widget_b = CountedOther {
            creates: Rc::clone(&creates_b),
        }
        .into_boxed_render_box();
        harness.update(&widget_a, &widget_b);
        widget_b.update_render_object(&harness.root.element, &mut slot.object);

        assert_eq!(
            creates_b.get(),
            1,
            "type change must recreate the render object"
        );
        assert!(
            (*slot.object)
                .as_any_mut()
                .downcast_mut::<RenderOther>()
                .is_some(),
            "slot now holds the new type"
        );
        assert!(
            (*slot.object).as_any_mut().downcast_mut::<()>().is_none(),
            "old type is gone"
        );
    }
}
