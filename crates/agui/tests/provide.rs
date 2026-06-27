use std::{cell::Cell, rc::Rc};

use agui::provide::Provide;
use agui_test::{ElementTester, fixtures::Leaf};

#[test]
fn a_descendant_reads_a_provided_value_at_mount() {
    let read = Rc::new(Cell::new(None));
    let recorder = Rc::clone(&read);
    let leaf = Leaf::new()
        .on_mount(move |ctx| recorder.set(ctx.get_provided::<usize>().as_deref().copied()));

    let _tester = ElementTester::mount(Provide::new(42usize).child(leaf));

    assert_eq!(
        read.get(),
        Some(42),
        "the descendant saw the provided value in scope"
    );
}

#[test]
fn an_absent_provided_value_reads_none() {
    let read = Rc::new(Cell::new(Some(0)));
    let recorder = Rc::clone(&read);
    let leaf = Leaf::new()
        .on_mount(move |ctx| recorder.set(ctx.get_provided::<usize>().as_deref().copied()));

    let _tester = ElementTester::mount(leaf);

    assert_eq!(read.get(), None, "no provider in scope, no value");
}

// A descendant that depends on the provided `usize` at mount and flags when its dependency-change hook
// runs. `Leaf` routes `dependency_changed` to `on_rebuild`.
fn dependent_reader(fired: Rc<Cell<bool>>) -> Leaf {
    Leaf::new()
        .on_mount(|ctx| {
            ctx.build(|ctx| {
                ctx.depend_on_provided::<usize>();
            });
        })
        .on_rebuild(move |_| fired.set(true))
}

#[test]
fn changing_a_provided_value_runs_a_dependents_dependency_change_hook() {
    let fired = Rc::new(Cell::new(false));

    let mut tester =
        ElementTester::mount(Provide::new(1usize).child(dependent_reader(Rc::clone(&fired))));
    assert!(
        !fired.get(),
        "mount registers the dependency but runs no change hook"
    );

    tester.rebuild(Provide::new(2usize).child(dependent_reader(Rc::clone(&fired))));
    assert!(
        fired.get(),
        "changing the value ran the dependent's dependency-change hook"
    );
}
