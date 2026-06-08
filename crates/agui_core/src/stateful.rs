use crate::{
    context::{Dispatch, UpdateCtx},
    element::{Element, RoutingId, node::ElementNode},
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

/// The [`Element`] of a [`Stateful`] widget. It owns the state and the child's materialized subtree.
pub struct StatefulElement<S: State> {
    state: S,
    child: ElementNode<<S::Child as Widget>::Element>,
}

impl<S> Element for StatefulElement<S>
where
    S: State,
    <S::Child as Widget>::Element: Element<Render = <S::Child as Widget>::Render>,
    <S::Child as Widget>::Render: 'static,
{
    type Render = <S::Child as Widget>::Render;

    fn dispatch(&mut self, render: &mut Self::Render, path: &[RoutingId], action: Dispatch) {
        let Some((_, rest)) = path.split_first() else {
            match action {
                Dispatch::Message(ctx) => {
                    let apply: SetState<S> = ctx.consume();
                    apply(&mut self.state);
                    ctx.request_rebuild();
                }

                Dispatch::Rebuild(ctx) => {
                    let child = self.state.build();

                    ctx.with_routing_id(RoutingId::from_index(0), |ctx| {
                        child.update(&mut self.child.element, render, ctx);
                    });
                }
            }

            return;
        };

        self.child.element.dispatch(render, rest, action);
    }
}

impl<S> Widget for Stateful<S>
where
    S: State,
    <S::Child as Widget>::Element: Element<Render = <S::Child as Widget>::Render>,
    <S::Child as Widget>::Render: 'static,
{
    type Element = StatefulElement<S>;

    type Render = <S::Child as Widget>::Render;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let state = self.initial;

        let child = state.build();
        let (child_element, child_render) =
            ctx.with_routing_id(RoutingId::from_index(0), |ctx| child.create(ctx));

        let element = StatefulElement {
            state,
            child: ElementNode::new(child_element),
        };

        (element, child_render)
    }

    fn update(self, element: &mut Self::Element, render: &mut Self::Render, ctx: &mut UpdateCtx) {
        // State lives on the element and persists, so the re-supplied initial is dropped. Rebuild from
        // the current state so a provided value that changed above reaches the child.
        let Self { initial: _ } = self;

        let child = element.state.build();

        ctx.with_routing_id(RoutingId::from_index(0), |ctx| {
            child.update(&mut element.child.element, render, ctx);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use typed_floats::{Positive, PositiveFinite};

    use super::*;
    use crate::{
        context::{LayoutCtx, MountCtx, PaintCtx},
        element::{LeafElement, RoutingPath},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        pipeline::build::BuildOwner,
        render_object::{
            RenderObject,
            box_layout::{BoxConstraints, RenderBox},
        },
        test_harness::{TestTaskRunner, with_ctx},
        text::TextBaseline,
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
        type Element = LeafElement<RenderSquare>;

        type Render = RenderSquare;

        fn create(self, _: &mut UpdateCtx) -> (LeafElement<RenderSquare>, RenderSquare) {
            self.creates.set(self.creates.get() + 1);

            (LeafElement::new(), RenderSquare { side: self.side })
        }

        fn update(
            self,
            _: &mut LeafElement<RenderSquare>,
            render: &mut RenderSquare,
            _: &mut UpdateCtx,
        ) {
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
        assert_eq!(render.borrow().side, 1.0);

        // A set-state delivered to the widget's own path mutates the state and asks for a rebuild.
        let bump: SetState<Counter> = Box::new(|state| state.count += 1);
        owner.dispatch_message(
            &RoutingPath::new(owner.root_id(), Vec::new()),
            Box::new(bump),
        );
        assert!(owner.is_dirty(), "set-state requested a rebuild");

        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(
            render.borrow().side,
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
