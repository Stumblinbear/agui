use agui::widget::AsAnyWidget;
use agui_test::{
    ElementTester,
    fixtures::{BoxedChildren, KeyedLeaf, ProbeLog},
};

fn logs(count: usize) -> Vec<ProbeLog> {
    (0..count).map(|_| ProbeLog::default()).collect()
}

#[test]
fn boxed_children_reorder_reuses_each_by_key() {
    let logs = logs(2);
    let mut tester = ElementTester::mount(BoxedChildren {
        children: vec![
            KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
        ],
    });

    // Reorder the runtime-mixed children; each is reused by key through the box.
    tester.rebuild(BoxedChildren {
        children: vec![
            KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
        ],
    });

    assert_eq!(logs[0].mounts.get(), 1, "boxed key 0 reused");
    assert_eq!(logs[1].mounts.get(), 1, "boxed key 1 reused");
    assert_eq!(logs[0].updates.get(), 1);
    assert_eq!(logs[1].updates.get(), 1);
}
