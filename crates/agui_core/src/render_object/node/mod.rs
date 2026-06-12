use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::PaintCtx,
    diagnostics::{Diagnostics, DiagnosticsNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        LayoutCtx, MountCtx, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
};

mod relayout_node;

pub use relayout_node::*;

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

    /// Captures the held render object's subtree, annotated with this holder's pipeline state.
    pub fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.decorate()
            .flag("parent_uses_size", self.parent_uses_size)
            .flag("needs_compositing", self.needs_compositing)
            .child(|d| self.object.describe(d))
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

    pub fn measure(&self, constraints: BoxConstraints) -> Size {
        self.object.measure(constraints)
    }

    /// Lay this child out under `constraints`. If you need the resulting size of the child, use `layout_and_get_size` instead.
    pub fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) {
        self.object.layout(ctx, constraints);
    }

    /// Lay this child out under `constraints` and return the size it took. This couples the child with the parent so that when
    /// the child's layout changes, the parent is also laid out.
    pub fn layout_and_get_size(
        &mut self,
        ctx: &mut LayoutCtx,
        constraints: BoxConstraints,
    ) -> Size {
        let size = self.object.layout(ctx, constraints);
        self.parent_uses_size = true;
        size
    }

    pub fn measure_baseline(
        &self,
        constraints: BoxConstraints,
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

impl<R, P> AsRef<R> for RenderNode<R, P> {
    fn as_ref(&self) -> &R {
        &self.object
    }
}

impl<R, P> AsMut<R> for RenderNode<R, P> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.object
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use typed_floats::{Positive, PositiveFinite, as_const};

    use super::*;
    use crate::{
        context::UpdateCtx,
        element::{Element, SingleChildElement},
        paint::compositing::{LayerHandle, OffsetLayer},
        pipeline::{layout::LayoutPipeline, paint::PaintPipeline},
        prelude::render_object::LayoutScope,
        render_object::{SingleChildRenderObject, box_layout::RenderBox},
        test_fixtures::Leaf,
        test_harness::TestCtx,
        text::TextBaseline,
        widget::{AsAnyWidget, Widget},
    };

    struct RenderPad<C: RenderBox> {
        pad: u32,
        child: RenderNode<C>,
    }

    impl<C: RenderBox> SingleChildRenderObject for RenderPad<C> {
        type Child = C;

        fn with_child<R>(&self, f: impl FnOnce(&C) -> R) -> R {
            f(&self.child.object)
        }

        fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut C) -> R) -> R {
            f(&mut self.child.object)
        }
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

        fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
            self.child.paint(ctx, offset);
        }
    }

    struct Pad<Child> {
        pad: u32,
        child: Child,
    }

    impl<Child: Widget> Widget for Pad<Child>
    where
        Child::Render: RenderBox,
    {
        type Element = SingleChildElement<Child::Element, RenderPad<Child::Render>>;

        type Render = RenderPad<Child::Render>;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let (element, child_render) = SingleChildElement::new(self.child, ctx);

            (
                element,
                RenderPad {
                    pad: self.pad,
                    child: RenderNode::new(child_render),
                },
            )
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            render.pad = self.pad;

            element.update(self.child, &mut render.child.object, ctx);
        }
    }

    #[test]
    fn pad_builds_and_updates_its_render() {
        let (mut element, mut render) = TestCtx::new().create(Pad {
            pad: 4,
            child: Leaf::new(),
        });
        assert_eq!(render.pad, 4);

        TestCtx::new().update(
            Pad {
                pad: 9,
                child: Leaf::new(),
            },
            &mut element,
            &mut render,
        );
        assert_eq!(render.pad, 9);
    }

    #[test]
    fn node_layout_and_paint_helpers() {
        let mut node = RenderNode::<(), ()>::new(());

        let layout = LayoutPipeline::default();
        let mut paint = PaintPipeline::default();

        let size = node.layout_and_get_size(
            &mut LayoutCtx::new(&layout, &mut paint, LayoutScope::detached()),
            BoxConstraints::tight(Size::new(10.0, 20.0)),
        );
        assert_eq!(size, Size::new(10.0, 20.0));

        PaintCtx::paint(&LayerHandle::new(OffsetLayer::new()), |ctx| {
            node.paint(ctx, Offset::ZERO);
        });
    }

    struct Counted {
        creates: Rc<Cell<usize>>,
    }

    struct CountedElement;

    impl Element for CountedElement {
        type Render = ();
    }

    impl Widget for Counted {
        type Element = CountedElement;

        type Render = ();

        fn create(self, _: &mut UpdateCtx) -> (CountedElement, ()) {
            self.creates.set(self.creates.get() + 1);

            (CountedElement, ())
        }

        fn update(self, _: &mut CountedElement, (): &mut Self::Render, _: &mut UpdateCtx) {}
    }

    #[test]
    fn boxed_slot_reuses_render_on_same_type() {
        let creates = Rc::new(Cell::new(0usize));

        let (mut element, mut render) = TestCtx::new().create(
            Counted {
                creates: Rc::clone(&creates),
            }
            .into_boxed_render_box(),
        );
        assert_eq!(creates.get(), 1);

        // Same concrete type: the boxed render object is reused, not recreated.
        TestCtx::new().update(
            Counted {
                creates: Rc::clone(&creates),
            }
            .into_boxed_render_box(),
            &mut element,
            &mut render,
        );
        assert_eq!(
            creates.get(),
            1,
            "same-type update reuses the boxed render object"
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

    struct CountedOther {
        creates: Rc<Cell<usize>>,
    }

    struct CountedOtherElement;

    impl Element for CountedOtherElement {
        type Render = RenderOther;
    }

    impl Widget for CountedOther {
        type Element = CountedOtherElement;

        type Render = RenderOther;

        fn create(self, _: &mut UpdateCtx) -> (CountedOtherElement, RenderOther) {
            self.creates.set(self.creates.get() + 1);

            (CountedOtherElement, RenderOther::default())
        }

        fn update(self, _: &mut CountedOtherElement, _: &mut RenderOther, _: &mut UpdateCtx) {}
    }

    #[test]
    fn boxed_slot_recreates_render_on_type_swap() {
        let creates_a = Rc::new(Cell::new(0usize));
        let (mut element, mut render) = TestCtx::new().create(
            Counted {
                creates: Rc::clone(&creates_a),
            }
            .into_boxed_render_box(),
        );
        assert_eq!(creates_a.get(), 1);
        assert!((*render).as_any_mut().downcast_mut::<()>().is_some());

        // Swap to a different concrete render type: the inner is recreated, not reused.
        let creates_b = Rc::new(Cell::new(0usize));
        TestCtx::new().update(
            CountedOther {
                creates: Rc::clone(&creates_b),
            }
            .into_boxed_render_box(),
            &mut element,
            &mut render,
        );

        assert_eq!(
            creates_b.get(),
            1,
            "type change recreates the render object"
        );
        assert!(
            (*render)
                .as_any_mut()
                .downcast_mut::<RenderOther>()
                .is_some(),
            "slot now holds the new type"
        );
        assert!(
            (*render).as_any_mut().downcast_mut::<()>().is_none(),
            "old type is gone"
        );
    }
}
