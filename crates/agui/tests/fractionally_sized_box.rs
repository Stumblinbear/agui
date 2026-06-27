use agui::widgets::fractionally_sized_box::FractionallySizedBox;
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn fractionally_sized_box_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| FractionallySizedBox::new().child(child));
}

#[test]
fn fractionally_sized_box_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| FractionallySizedBox::new().child(child));
}
