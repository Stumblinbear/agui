use crate::{
    context::Dispatch,
    element::{Element, RoutingId, node::ElementNode},
    prelude::element::UpdateCtx,
    widget::Widget,
};

/// State that persists across rebuilds and produces the subtree to show for it.
///
/// Implement this on the data a widget owns; [`build`](Self::build) reads that data and returns the
/// child to display.
pub trait WidgetState {
    type Widget: Widget;

    type Child: Widget;

    fn init_state(ctx: &mut UpdateCtx, widget: Self::Widget) -> Self;

    fn did_update_widget(&mut self, ctx: &mut UpdateCtx, widget: Self::Widget);

    /// Builds the subtree to show for the current state.
    fn build(&self, ctx: &mut UpdateCtx) -> Self::Child;
}

/// A mutation applied to a [`State`] to change it, delivered as a message to a [`Stateful`] widget.
///
/// Deliver one to the widget's path to mutate its state and schedule a rebuild of its subtree.
pub type SetState<S> = Box<dyn FnOnce(&mut S)>;

/// The [`Element`] of a [`Stateful`] widget. It owns the state and the child's materialized subtree.
pub struct StatefulElement<S>
where
    S: WidgetState,
{
    state: S,

    child: ElementNode<<S::Child as Widget>::Element>,
}

impl<S> StatefulElement<S>
where
    S: WidgetState,
{
    pub fn new(state: S, child: <S::Child as Widget>::Element) -> Self {
        Self {
            state,

            child: ElementNode::new(child),
        }
    }
}

impl<S> Element for StatefulElement<S>
where
    S: WidgetState + 'static,
    S::Child: Widget,
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
                    let child = self.state.build(ctx);
                    child.update(&mut self.child.element, render, ctx);
                }
            }

            return;
        };

        self.child.element.dispatch(render, rest, action);
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
        prelude::element::UpdateCtx,
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

    impl Widget for Counter {
        type Element = StatefulElement<CounterState>;

        type Render = <<CounterState as WidgetState>::Child as Widget>::Render;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let state = CounterState::init_state(ctx, self);
            let child = state.build(ctx);
            let (child_element, child_render) = child.create(ctx);
            let element = StatefulElement::new(state, child_element);
            (element, child_render)
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.state.did_update_widget(ctx, self);
            let child = element.state.build(ctx);
            child.update(&mut element.child.element, render, ctx);
        }
    }

    struct CounterState {
        count: u32,

        creates: Rc<Cell<usize>>,
    }

    impl WidgetState for CounterState {
        type Widget = Counter;
        type Child = Square;

        fn init_state(_: &mut UpdateCtx, widget: Self::Widget) -> Self {
            CounterState {
                count: widget.count,
                creates: widget.creates,
            }
        }

        fn did_update_widget(&mut self, _: &mut UpdateCtx, widget: Self::Widget) {
            self.count = widget.count;
            self.creates = widget.creates;
        }

        #[allow(clippy::cast_precision_loss)]
        fn build(&self, _: &mut UpdateCtx) -> Self::Child {
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
            Counter {
                count: 1,

                creates: Rc::clone(&creates),
            },
            &mut tasks.scheduler(),
        );

        assert_eq!(creates.get(), 1);
        assert_eq!(render.borrow().side, 1.0);

        // A set-state delivered to the widget's own path mutates the state and asks for a rebuild.
        let bump: SetState<CounterState> = Box::new(|state| state.count += 1);
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

        let (_element, mut render) = with_ctx(|ctx| {
            Counter {
                count: 5,

                creates: Rc::clone(&creates),
            }
            .create(ctx)
        });

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
