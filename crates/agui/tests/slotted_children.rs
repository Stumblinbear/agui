use agui::widget::AsAnyWidget;
use agui_test::{
    ElementTester,
    fixtures::{KeyedLeaf, ProbeLog, SlottedChildren, TileSlot},
};

fn logs(count: usize) -> Vec<ProbeLog> {
    (0..count).map(|_| ProbeLog::default()).collect()
}

#[test]
fn a_slot_child_reconciles_in_place() {
    let logs = logs(2);
    let mut tester = ElementTester::mount(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Title,
                KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            ),
        ],
    });

    tester.rebuild(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Title,
                KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            ),
        ],
    });

    assert_eq!(logs[0].mounts.get(), 1, "leading reused in place");
    assert_eq!(logs[1].mounts.get(), 1, "title reused in place");
    assert_eq!(logs[0].updates.get(), 1);
    assert_eq!(logs[1].updates.get(), 1);
    assert_eq!(logs[0].unmounts.get(), 0);
    assert_eq!(logs[1].unmounts.get(), 0);
}

#[test]
fn an_unfilled_slot_unmounts_and_a_new_slot_mounts() {
    let logs = logs(3);
    let mut tester = ElementTester::mount(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Title,
                KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            ),
        ],
    });

    tester.rebuild(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Trailing,
                KeyedLeaf::new(2, logs[2].clone()).into_boxed_render_box(),
            ),
        ],
    });

    assert_eq!(logs[0].updates.get(), 1, "leading reused in place");
    assert_eq!(logs[0].unmounts.get(), 0);
    assert_eq!(logs[1].unmounts.get(), 1, "unfilled title unmounted");
    assert_eq!(logs[2].mounts.get(), 1, "new trailing mounted");
}

#[test]
fn a_key_change_replaces_the_slot_child() {
    let old_log = ProbeLog::default();
    let new_log = ProbeLog::default();

    let mut tester = ElementTester::mount(SlottedChildren {
        children: vec![(
            TileSlot::Leading,
            KeyedLeaf::new(0, old_log.clone()).into_boxed_render_box(),
        )],
    });

    tester.rebuild(SlottedChildren {
        children: vec![(
            TileSlot::Leading,
            KeyedLeaf::new(9, new_log.clone()).into_boxed_render_box(),
        )],
    });

    assert_eq!(old_log.unmounts.get(), 1, "key mismatch replaces the child");
    assert_eq!(old_log.updates.get(), 0);
    assert_eq!(new_log.mounts.get(), 1);
}

#[test]
fn children_swapped_between_slots_stay_with_their_slots() {
    let logs = logs(2);
    let mut tester = ElementTester::mount(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Title,
                KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            ),
        ],
    });

    // Exchange the widgets between the two slots. Slot identity keys the reconcile, and the keys differ, so
    // each slot replaces its child rather than following the widget across.
    tester.rebuild(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Title,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
        ],
    });

    assert_eq!(
        logs[0].mounts.get(),
        2,
        "key 0 remounted under its new slot"
    );
    assert_eq!(
        logs[1].mounts.get(),
        2,
        "key 1 remounted under its new slot"
    );
    assert_eq!(logs[0].unmounts.get(), 1);
    assert_eq!(logs[1].unmounts.get(), 1);
}

#[test]
#[should_panic(expected = "same slot")]
fn assigning_a_slot_twice_panics() {
    let logs = logs(2);
    ElementTester::mount(SlottedChildren {
        children: vec![
            (
                TileSlot::Leading,
                KeyedLeaf::new(0, logs[0].clone()).into_boxed_render_box(),
            ),
            (
                TileSlot::Leading,
                KeyedLeaf::new(1, logs[1].clone()).into_boxed_render_box(),
            ),
        ],
    });
}
