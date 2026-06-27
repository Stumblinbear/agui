use std::{cell::Cell, rc::Rc};

use agui::{
    context::{BuildCtx, CreateCtx, UpdateCtx},
    provide::Provide,
    stateful::{SetState, StatefulElement, WidgetState},
    widget::Widget,
};
use agui_test::{ElementTester, fixtures::Leaf};

// State holding a count, building a child that writes the count to a shared cell when it reconciles, so a
// test sees which count the rebuilt child carried.
struct Counter {
    count: u32,
    observed: Rc<Cell<u32>>,
}

impl WidgetState for Counter {
    type Widget = CounterWidget;
    type Child = Leaf;

    fn init_state(_: &mut BuildCtx, widget: Self::Widget) -> Self {
        Counter {
            count: widget.count,
            observed: widget.observed,
        }
    }

    fn did_update_widget(&mut self, _: &mut BuildCtx, widget: Self::Widget) {
        self.count = widget.count;
        self.observed = widget.observed;
    }

    fn build(&self, _: &mut BuildCtx) -> Self::Child {
        let count = self.count;
        let observed = Rc::clone(&self.observed);
        Leaf::new().on_update(move |_| observed.set(count))
    }
}

struct CounterWidget {
    count: u32,
    observed: Rc<Cell<u32>>,
}

impl Widget for CounterWidget {
    type Element = StatefulElement<Counter>;

    type Render = ();

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let state = ctx.build(|ctx| Counter::init_state(ctx, self));
        StatefulElement::new(state)
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.update_widget(ctx, self);
    }
}

#[test]
fn set_state_rebuilds_the_child() {
    let observed = Rc::new(Cell::new(0));
    let mut tester = ElementTester::mount(CounterWidget {
        count: 0,
        observed: Rc::clone(&observed),
    });
    let handle = tester.root_handle();

    let bump: SetState<Counter> = Box::new(|state| state.count = 7);
    tester.dispatch(handle, Box::new(bump));
    assert_eq!(observed.get(), 7);

    let bump: SetState<Counter> = Box::new(|state| state.count = 12);
    tester.dispatch(handle, Box::new(bump));
    assert_eq!(observed.get(), 12);
}

// A stateful consumer of a provided `usize`. It records the value its build saw and counts its
// dependency-change hook, so a test can confirm a `Provide` change runs `did_change_dependencies` and
// rebuilds the consumer with the new value.
struct Consumer {
    seen: Rc<Cell<usize>>,
    dependency_changes: Rc<Cell<u32>>,
}

impl WidgetState for Consumer {
    type Widget = ConsumerWidget;
    type Child = Leaf;

    fn init_state(_: &mut BuildCtx, widget: Self::Widget) -> Self {
        Consumer {
            seen: widget.seen,
            dependency_changes: widget.dependency_changes,
        }
    }

    fn did_update_widget(&mut self, _: &mut BuildCtx, widget: Self::Widget) {
        self.seen = widget.seen;
        self.dependency_changes = widget.dependency_changes;
    }

    fn did_change_dependencies(&mut self, _: &mut BuildCtx) {
        self.dependency_changes
            .set(self.dependency_changes.get() + 1);
    }

    fn build(&self, ctx: &mut BuildCtx) -> Self::Child {
        let value = ctx
            .depend_on_provided::<usize>()
            .as_deref()
            .copied()
            .unwrap_or(0);
        self.seen.set(value);
        Leaf::new()
    }
}

struct ConsumerWidget {
    seen: Rc<Cell<usize>>,
    dependency_changes: Rc<Cell<u32>>,
}

impl Widget for ConsumerWidget {
    type Element = StatefulElement<Consumer>;

    type Render = ();

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let state = ctx.build(|ctx| Consumer::init_state(ctx, self));
        StatefulElement::new(state)
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        element.update_widget(ctx, self);
    }
}

#[test]
fn a_dependency_change_rebuilds_a_stateful_consumer_with_the_new_value() {
    let seen = Rc::new(Cell::new(0));
    let dependency_changes = Rc::new(Cell::new(0));

    let mut tester = ElementTester::mount(Provide::new(1usize).child(ConsumerWidget {
        seen: Rc::clone(&seen),
        dependency_changes: Rc::clone(&dependency_changes),
    }));
    assert_eq!(
        seen.get(),
        1,
        "the consumer read the value when it built at mount"
    );
    assert_eq!(dependency_changes.get(), 0, "no dependency change at mount");

    tester.rebuild(Provide::new(2usize).child(ConsumerWidget {
        seen: Rc::clone(&seen),
        dependency_changes: Rc::clone(&dependency_changes),
    }));
    assert_eq!(seen.get(), 2, "the consumer rebuilt with the new value");
    assert_eq!(
        dependency_changes.get(),
        1,
        "its dependency-change hook ran once"
    );
}
