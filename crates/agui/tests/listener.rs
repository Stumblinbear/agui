use agui::widgets::listener::Listener;
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn listener_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| Listener::builder().child(child));
}

#[test]
fn listener_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| Listener::builder().child(child));
}
