use agui::widgets::intrinsic_width::IntrinsicWidth;
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn intrinsic_width_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| IntrinsicWidth::builder().child(child));
}

#[test]
fn intrinsic_width_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| IntrinsicWidth::builder().child(child));
}
