use agui::widgets::center::Center;
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn center_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| Center::new().child(child));
}

#[test]
fn center_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| Center::new().child(child));
}
