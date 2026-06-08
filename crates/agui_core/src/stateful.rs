use std::{cell::RefCell, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::{Dispatch, LayoutCtx, MountCtx, PaintCtx, UpdateCtx},
    element::{Element, RoutingId, node::ElementNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    render_object::{
        RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
    widget::Widget,
};

/// State that persists across rebuilds and produces the subtree to show for it.
///
/// Implement this on the data a widget owns; [`build`](Self::build) reads that data and returns the
/// child to display. A [`Stateful`] widget holds the state on its element, so it survives rebuilds, and
/// reconciles the child returned by `build` whenever the state changes.
pub trait State: 'static {
    /// The widget [`build`](Self::build) returns. Its type is fixed across rebuilds; use a boxed widget
    /// for a subtree whose shape varies.
    type Child: Widget;

    /// Builds the subtree to show for the current state.
    fn build(&self) -> Self::Child;
}

/// A mutation applied to a [`State`] to change it, delivered as a message to a [`Stateful`] widget.
///
/// Deliver one to the widget's path to mutate its state and schedule a rebuild of its subtree.
pub type SetState<S> = Box<dyn FnOnce(&mut S)>;

/// A widget whose [`State`] persists across rebuilds and whose subtree is reconciled when the state
/// changes.
///
/// Mount it with an initial state. A [`SetState`] message delivered to its path mutates the state and
/// rebuilds the subtree in place, keeping the child's element and render object where their type is
/// unchanged. The widget reports its child's size as its own; it draws nothing of its own.
pub struct Stateful<S> {
    initial: S,
}

impl<S: State> Stateful<S> {
    /// A stateful widget starting from `initial`.
    pub fn new(initial: S) -> Self {
        Self { initial }
    }
}

/// The [`Element`] of a [`Stateful`] widget. It owns the state and the child's materialized subtree, and
/// reconciles the child against the state on each rebuild.
pub struct StatefulElement<S: State> {
    state: S,
    child: ElementNode<<S::Child as Widget>::Element>,

    /// The child's render object, shared with the render tree so a rebuild reconciles it from here.
    render: Rc<RefCell<<S::Child as Widget>::Render>>,
}

impl<S> Element for StatefulElement<S>
where
    S: State,
    <S::Child as Widget>::Element: 'static,
    <S::Child as Widget>::Render: 'static,
{
    fn dispatch(&mut self, path: &[RoutingId], action: Dispatch) {
        let Some((_, rest)) = path.split_first() else {
            match action {
                Dispatch::Message(ctx) => {
                    let apply: SetState<S> = ctx.consume();
                    apply(&mut self.state);
                    ctx.request_rebuild();
                }

                Dispatch::Rebuild(ctx) => {
                    let child = self.state.build();
                    let mut render = self.render.borrow_mut();

                    ctx.with_routing_id(RoutingId::from_index(0), |ctx| {
                        child.update(&mut self.child.element, &mut render, ctx);
                    });
                }
            }

            return;
        };

        self.child.element.dispatch(rest, action);
    }
}

impl<S> Widget for Stateful<S>
where
    S: State,
    <S::Child as Widget>::Element: 'static,
    <S::Child as Widget>::Render: RenderBox + 'static,
{
    type Element = StatefulElement<S>;

    type Render = StatefulRender<<S::Child as Widget>::Render>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let state = self.initial;

        let child = state.build();
        let (child_element, child_render) =
            ctx.with_routing_id(RoutingId::from_index(0), |ctx| child.create(ctx));

        let render = Rc::new(RefCell::new(child_render));

        let element = StatefulElement {
            state,
            child: ElementNode::new(child_element),
            render: Rc::clone(&render),
        };

        (element, StatefulRender { child: render })
    }

    fn update(self, element: &mut Self::Element, render: &mut Self::Render, ctx: &mut UpdateCtx) {
        // State lives on the element and persists, so the re-supplied initial is dropped. Rebuild from
        // the current state so a provided value that changed above reaches the child.
        let Self { initial: _ } = self;

        let child = element.state.build();

        ctx.with_routing_id(RoutingId::from_index(0), |ctx| {
            child.update(
                &mut element.child.element,
                &mut render.child.borrow_mut(),
                ctx,
            );
        });
    }
}

/// The render object of a [`Stateful`] widget: it presents its child unchanged, sharing the child's
/// render object with the element so a rebuild reconciles the same object the render tree lays out.
pub struct StatefulRender<R> {
    child: Rc<RefCell<R>>,
}

impl<R: RenderObject> RenderObject for StatefulRender<R> {
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.child.borrow_mut().mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut MountCtx) {
        self.child.borrow_mut().unmount(ctx);
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.borrow_mut().update_compositing_bits()
    }
}

impl<R: RenderBox> RenderBox for StatefulRender<R> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.borrow().min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.borrow().max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.borrow().min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.borrow().max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child.borrow().measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.child.borrow_mut().layout(ctx, constraints)
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.borrow().measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.borrow_mut().distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.child.borrow().hit_test(result, position)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.borrow_mut().paint(ctx, offset);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::{
        element::RoutingPath,
        pipeline::build::BuildOwner,
        test_harness::{TestTaskRunner, with_ctx},
    };

    /// State holding a count, building a square sized to it. `creates` tallies how many child render
    /// objects were built, so a test can tell a reconcile (reused) from a replacement (rebuilt).
    struct Counter {
        count: u32,
        creates: Rc<Cell<usize>>,
    }

    impl State for Counter {
        type Child = Square;

        #[allow(clippy::cast_precision_loss)]
        fn build(&self) -> Square {
            Square {
                side: self.count as f32,
                creates: Rc::clone(&self.creates),
            }
        }
    }

    /// A leaf laying out to a square of `side`, whose render object records its own creation.
    struct Square {
        side: f32,
        creates: Rc<Cell<usize>>,
    }

    impl Widget for Square {
        type Element = ();

        type Render = RenderSquare;

        fn create(self, _: &mut UpdateCtx) -> ((), RenderSquare) {
            self.creates.set(self.creates.get() + 1);

            ((), RenderSquare { side: self.side })
        }

        fn update(self, (): &mut (), render: &mut RenderSquare, _: &mut UpdateCtx) {
            render.side = self.side;
        }
    }

    struct RenderSquare {
        side: f32,
    }

    impl RenderObject for RenderSquare {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for RenderSquare {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.constrain(Size::new(self.side, self.side))
        }

        fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            constraints.constrain(Size::new(self.side, self.side))
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

    #[test]
    #[allow(clippy::float_cmp)]
    fn set_state_reconciles_the_child_render_in_place() {
        let creates = Rc::new(Cell::new(0));

        let mut tasks = TestTaskRunner::new();
        let (mut owner, render) = BuildOwner::mount(
            Stateful::new(Counter {
                count: 1,
                creates: Rc::clone(&creates),
            }),
            &mut tasks.scheduler(),
        );

        assert_eq!(creates.get(), 1);
        assert_eq!(render.borrow().child.borrow().side, 1.0);

        // A set-state delivered to the widget's own path mutates the state and asks for a rebuild.
        let bump: SetState<Counter> = Box::new(|state| state.count += 1);
        owner.dispatch_message(
            &RoutingPath::new(owner.root_id(), Vec::new()),
            Box::new(bump),
        );
        assert!(owner.is_dirty(), "set-state requested a rebuild");

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(
            render.borrow().child.borrow().side,
            2.0,
            "the rebuild reconciled the child render to the new state"
        );
        assert_eq!(
            creates.get(),
            1,
            "the same-type child reused its render object rather than rebuilding it"
        );
    }

    #[test]
    fn the_wrapper_presents_its_child_unchanged() {
        let creates = Rc::new(Cell::new(0));

        let (_element, mut render) =
            with_ctx(|ctx| Stateful::new(Counter { count: 5, creates }).create(ctx));

        let size = render.layout(
            &mut LayoutCtx::detached(),
            BoxConstraints::loose(Size::new(100, 100)),
        );

        assert_eq!(
            size,
            Size::new(5.0, 5.0),
            "the wrapper reports its child's size"
        );
    }
}
