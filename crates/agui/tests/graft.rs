//! A structural graft must re-lay the subtree it swaps in. These drive full frames through the harness, so
//! they live here rather than in `#[cfg(test)]` modules: the harness (`agui_test`) depends on `agui`, and only
//! an external test crate links the one `agui` lib both sides share.

use std::time::Duration;

use agui::{
    geometry::Size,
    key::Key,
    widget::AsAnyWidget,
    widgets::{flex::Column, sized_box::SizedBox},
};
use agui_test::{
    Probe, WidgetTester,
    fixtures::{IntrinsicBox, TestBox},
};

#[test]
fn a_keyed_child_is_laid_out_after_a_graft() {
    let probe = Probe::new();
    let child = || probe.wrap(TestBox::new(Size::new(10, 10)));

    let mut tester = WidgetTester::mount(Key::value(1u32).child(child()));
    tester.resize(Size::new(10, 10));
    tester.pump(Duration::ZERO);
    assert_eq!(probe.layouts(), 1, "the initial child is laid out once");

    // A changed key grafts a fresh child, which must be laid out even though only the key changed.
    tester.rebuild(Key::value(2u32).child(child()));
    assert_eq!(
        probe.layouts(),
        2,
        "the child grafted under the new key must be laid out"
    );
}

#[test]
fn a_grown_child_is_laid_out() {
    let probe = Probe::new();
    let children = |n: usize| {
        (0..n)
            .map(|_| probe.wrap(TestBox::new(Size::new(10, 10))))
            .collect::<Vec<_>>()
    };

    let mut tester = WidgetTester::mount(Column::builder().children(children(2)).build());
    tester.resize(Size::new(50, 50));
    tester.pump(Duration::ZERO);
    let after_mount = probe.layouts();

    // Growing the list grafts a new child, which forces the list to re-lay; the shared probe sees more layouts.
    tester.rebuild(Column::builder().children(children(3)).build());
    assert!(
        probe.layouts() > after_mount,
        "growing the list must re-lay it so the new child is laid out"
    );
}

#[test]
fn an_erased_type_swap_is_laid_out() {
    let probe = Probe::new();
    let test_box = || {
        probe
            .wrap(TestBox::new(Size::new(10, 10)))
            .into_boxed_render_box()
    };
    let intrinsic_box = || {
        probe
            .wrap(IntrinsicBox::new(Size::new(10, 10)))
            .into_boxed_render_box()
    };

    let mut tester = WidgetTester::mount(SizedBox::new().width(50).height(50).child(test_box()));
    tester.resize(Size::new(50, 50));
    tester.pump(Duration::ZERO);
    let after_mount = probe.layouts();

    // A single-child wrapper reconciles its boxed child in place, so a concrete-type swap reaches the erased
    // element's own graft rather than a list replace; the grafted replacement must be laid out.
    tester.rebuild(SizedBox::new().width(50).height(50).child(intrinsic_box()));
    assert!(
        probe.layouts() > after_mount,
        "a boxed child that swaps type must be laid out"
    );
}
