use agui::key::Key;
use agui_test::{
    ElementTester,
    fixtures::{KeyedLeaf, ProbeLog},
};

#[test]
fn an_unchanged_key_reconciles_the_child_in_place() {
    let log = ProbeLog::default();

    let mut tester = ElementTester::mount(Key::value(1u32).child(KeyedLeaf::new(1, log.clone())));
    assert_eq!(log.mounts.get(), 1);
    assert_eq!(log.unmounts.get(), 0);

    tester.rebuild(Key::value(1u32).child(KeyedLeaf::new(1, log.clone())));
    assert_eq!(
        log.mounts.get(),
        1,
        "an unchanged key reuses the child rather than remounting it"
    );
    assert_eq!(log.unmounts.get(), 0);
    assert_eq!(log.updates.get(), 1, "the child is reconciled in place");
}

#[test]
fn a_changed_key_replaces_the_child() {
    let log = ProbeLog::default();

    let mut tester = ElementTester::mount(Key::value(1u32).child(KeyedLeaf::new(1, log.clone())));
    assert_eq!(log.mounts.get(), 1);
    assert_eq!(log.unmounts.get(), 0);

    tester.rebuild(Key::value(2u32).child(KeyedLeaf::new(1, log.clone())));
    assert_eq!(
        log.unmounts.get(),
        1,
        "a changed key unmounts the old child"
    );
    assert_eq!(
        log.mounts.get(),
        2,
        "and mounts a fresh child under the new key"
    );
}

#[test]
fn a_message_reaches_the_keyed_child() {
    let log = ProbeLog::default();

    let mut tester = ElementTester::mount(Key::value(1u32).child(KeyedLeaf::new(1, log.clone())));
    let handle = log
        .handle
        .get()
        .expect("the child records its handle at mount");

    tester.dispatch(handle, Box::new(7u32));
    assert_eq!(log.received.get(), Some(7));
}
