use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::UpdateCtx,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::{RenderObject, box_layout::BoxLayout},
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
};

pub struct RenderNode<R, P = ()> {
    pub object: R,
    pub parent_data: P,

    size: Option<Size>,
    parent_uses_size: bool,
}

impl<R, P: Default> RenderNode<R, P> {
    pub fn new(object: R) -> Self {
        Self {
            object,
            parent_data: P::default(),

            size: None,
            parent_uses_size: false,
        }
    }
}

impl<R, P> RenderNode<R, P> {
    pub fn new_with(object: R, parent_data: P) -> Self {
        Self {
            object,
            parent_data,

            size: None,
            parent_uses_size: false,
        }
    }
}

impl<R: RenderObject, P> RenderNode<R, P> {
    pub fn mount(&mut self, ctx: &mut UpdateCtx) {
        self.object.mount(ctx);
    }

    pub fn unmount(&mut self, ctx: &mut UpdateCtx) {
        self.object.unmount(ctx);
    }

    pub fn paint(&mut self, canvas: &mut Canvas) {
        self.object.paint(canvas);
    }

    pub fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.object.hit_test(result, position)
    }
}

impl<R: BoxLayout, P> RenderNode<R, P> {
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
    pub fn layout(&mut self, constraints: Constraints) {
        self.size = Some(self.object.layout(constraints));
    }

    /// Lay this child out under `constraints` and return the size it took. This couples the child with the parent so that when
    /// the child's layout changes, the parent is also laid out.
    pub fn layout_and_get_size(&mut self, constraints: Constraints) -> Size {
        let size = self.object.layout(constraints);
        self.size = Some(size);
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
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::*;
    use crate::{
        context::UpdateCtx,
        element::Element,
        render_object::{
            RenderLeaf,
            box_layout::{AnyRenderBox, BoxLayout},
        },
        test_fixtures::Leaf,
        test_harness::TestHarness,
        text_baseline::TextBaseline,
        view::{AsAnyView, View},
    };

    struct RenderPad<C: RenderObject> {
        pad: u32,
        child: RenderNode<C>,
    }

    impl<C: RenderObject> RenderObject for RenderPad<C> {
        fn mount(&mut self, _: &mut UpdateCtx) {}

        fn unmount(&mut self, _: &mut UpdateCtx) {}

        fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
            self.child.hit_test(result, position)
        }

        fn paint(&mut self, canvas: &mut Canvas) {
            self.child.paint(canvas);
        }
    }

    struct Pad<Child> {
        pad: u32,
        child: Child,
    }

    impl<Child: View> View for Pad<Child> {
        type State = ();

        type Render = RenderPad<Child::Render>;

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![Element::new(&self.child, ctx)], ())
        }

        fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
            element.child_mut(0, &old.child).update(&self.child, ctx);
        }

        fn create_render_object(&self, element: &Element) -> Self::Render {
            RenderPad {
                pad: self.pad,
                child: RenderNode::new(self.child.create_render_object(&element.children[0])),
            }
        }

        fn update_render_object(&self, element: &Element, object: &mut Self::Render) {
            object.pad = self.pad;

            self.child
                .update_render_object(&element.children[0], &mut object.child.object);
        }
    }

    #[test]
    fn create_render_object_builds_wrapped_subtree() {
        let view = Pad {
            pad: 4,
            child: Leaf::new(),
        };
        let harness = TestHarness::mount(&view);

        let node = RenderNode::<_, ()>::new(view.create_render_object(&harness.root));

        assert_eq!(node.object.pad, 4);
    }

    #[test]
    fn update_render_object_syncs_in_place() {
        let view = Pad {
            pad: 4,
            child: Leaf::new(),
        };

        let harness = TestHarness::mount(&view);
        let mut node = RenderNode::<_, ()>::new(view.create_render_object(&harness.root));

        assert_eq!(node.object.pad, 4);

        Pad {
            pad: 9,
            child: Leaf::new(),
        }
        .update_render_object(&harness.root, &mut node.object);

        assert_eq!(node.object.pad, 9);
    }

    #[test]
    fn node_layout_and_paint_helpers() {
        let mut node = RenderNode::<RenderLeaf, ()>::new(RenderLeaf::default());

        let size = node.layout_and_get_size(Constraints::tight(Size::new(10.0, 20.0)));
        assert_eq!(size, Size::new(10.0, 20.0));

        let mut canvas = Canvas {};
        node.paint(&mut canvas);
    }

    #[test]
    fn build_update_paint_layout_and_reconcile_a_boxed_child() {
        // construct a render tree through the real View seam
        let view = Pad {
            pad: 4,
            child: Leaf::new(),
        };
        let harness = TestHarness::mount(&view);
        let mut node = RenderNode::<_, ()>::new(view.create_render_object(&harness.root));
        assert_eq!(node.object.pad, 4);

        // update it in place
        Pad {
            pad: 9,
            child: Leaf::new(),
        }
        .update_render_object(&harness.root, &mut node.object);
        assert_eq!(node.object.pad, 9);

        // paint the whole tree; lay out the (BoxLayout) leaf child via the node helpers
        let mut canvas = Canvas {};
        node.paint(&mut canvas);
        assert_eq!(
            node.object
                .child
                .layout_and_get_size(Constraints::tight(Size::new(12.0, 8.0))),
            Size::new(12.0, 8.0),
        );

        // erased boundary: a boxed child, reconciled by recovering its real type —
        // this is what a fan-out's update_render_object does per slot.
        let mut boxed: RenderNode<Box<dyn AnyRenderBox>, ()> =
            RenderNode::new(Box::new(RenderLeaf::default()) as Box<dyn AnyRenderBox>);

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
            .downcast_mut::<RenderLeaf>()
            .expect("boxed child downcasts to its real type");

        assert_eq!(
            boxed.layout_and_get_size(Constraints::tight(Size::new(7.0, 7.0))),
            Size::new(7.0, 7.0)
        );

        // the boxed node still drives layout/paint through dyn dispatch
        assert_eq!(
            boxed.layout_and_get_size(Constraints::tight(Size::new(3.0, 3.0))),
            Size::new(3.0, 3.0),
        );

        boxed.paint(&mut canvas);
    }

    struct Counted {
        creates: Rc<Cell<usize>>,
    }

    impl View for Counted {
        type State = ();

        type Render = RenderLeaf;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &Element) -> Self::Render {
            self.creates.set(self.creates.get() + 1);

            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn updating_a_boxed_slot_reuses_the_render_object() {
        let creates = Rc::new(Cell::new(0usize));

        let view = Counted {
            creates: Rc::clone(&creates),
        }
        .into_boxed_render_box();

        let harness = TestHarness::mount(&view);

        // a fan-out slot: a boxed child wrapped in a RenderNode
        let mut slot: RenderNode<Box<dyn AnyRenderBox>, ()> =
            RenderNode::new(view.create_render_object(&harness.root));

        assert_eq!(creates.get(), 1);

        // same concrete type -> reuse the boxed render object, do not recreate
        view.update_render_object(&harness.root, &mut slot.object);

        assert_eq!(
            creates.get(),
            1,
            "same-type update must reuse the boxed render object, not recreate it"
        );
    }

    #[derive(Default)]
    struct RenderOther {}

    impl RenderObject for RenderOther {
        fn mount(&mut self, _: &mut UpdateCtx) {}

        fn unmount(&mut self, _: &mut UpdateCtx) {}

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, _: &mut Canvas) {}
    }

    impl BoxLayout for RenderOther {
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

        fn layout(&mut self, constraints: Constraints) -> Size {
            constraints.smallest()
        }

        fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }
    }

    struct CountedOther {
        creates: Rc<Cell<usize>>,
    }

    impl View for CountedOther {
        type Render = RenderOther;

        type State = ();

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &Element) -> Self::Render {
            self.creates.set(self.creates.get() + 1);

            RenderOther::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn type_swap_at_a_boxed_slot_recreates_the_render_object() {
        let creates_a = Rc::new(Cell::new(0usize));
        let view_a = Counted {
            creates: Rc::clone(&creates_a),
        }
        .into_boxed_render_box();
        let harness = TestHarness::mount(&view_a);

        let mut slot: RenderNode<Box<dyn AnyRenderBox>, ()> =
            RenderNode::new(view_a.create_render_object(&harness.root));
        assert_eq!(creates_a.get(), 1);
        assert!(
            (*slot.object)
                .as_any_mut()
                .downcast_mut::<RenderLeaf>()
                .is_some()
        );

        // swap to a different concrete render type -> downcast fails -> recreate
        let creates_b = Rc::new(Cell::new(0usize));
        let view_b = CountedOther {
            creates: Rc::clone(&creates_b),
        }
        .into_boxed_render_box();
        view_b.update_render_object(&harness.root, &mut slot.object);

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
            (*slot.object)
                .as_any_mut()
                .downcast_mut::<RenderLeaf>()
                .is_none(),
            "old type is gone"
        );
    }
}
