use std::{rc::Rc, time::Duration};

use agui::{
    render_object::box_layout::BoxConstraints,
    semantics::{Role, Semantics},
};
use agui_test::{WidgetTester, fixtures::RecordingBox};

#[test]
fn view_semantics_exposes_a_labeled_node() {
    let tester = WidgetTester::mount(
        Semantics::new()
            .role(Role::Button)
            .label("Submit")
            .child(()),
    );

    let tree = tester.semantics();
    let node = tree.find_by_name("Submit").expect("the labeled node");
    assert_eq!(node.config.node.role(), Role::Button);
}

#[test]
fn a_button_is_named_from_its_subtree() {
    let tester = WidgetTester::mount(
        Semantics::new()
            .role(Role::Button)
            .child(Semantics::new().role(Role::Label).label("Submit").child(())),
    );

    let tree = tester.semantics();
    let button = tree
        .find_by_name("Submit")
        .expect("the button named from its subtree");
    assert_eq!(button.config.node.role(), Role::Button);
    assert_eq!(button.accessible_name().as_deref(), Some("Submit"));
}

#[test]
fn a_rebuild_marks_the_boundary_and_the_flush_clears_it() {
    let render = RecordingBox::new();
    let builds = Rc::clone(&render.semantics_builds);

    let mut tester =
        WidgetTester::mount(Semantics::new().role(Role::Button).label("A").child(render));
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);

    tester.flush_semantics();
    let walked = builds.get();
    assert!(walked >= 1, "the boundary was walked once");

    tester.flush_semantics();
    assert_eq!(
        builds.get(),
        walked,
        "an unchanged boundary is not re-walked"
    );

    // Re-driving the tree with a different label marks the boundary. The mark survives the rebuild dispatch
    // only because the root re-enters the semantics scope it captured at mount.
    tester.rebuild(
        Semantics::new()
            .role(Role::Button)
            .label("B")
            .child(RecordingBox::new()),
    );
    tester.flush_semantics();
    let rewalked = builds.get();
    assert!(rewalked > walked, "the rebuilt boundary is re-walked");

    tester.flush_semantics();
    assert_eq!(builds.get(), rewalked, "the flush cleared the dirty set");
}
