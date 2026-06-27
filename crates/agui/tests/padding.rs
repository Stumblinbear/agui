use agui::{geometry::EdgeInsets, widgets::padding::Padding};
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn padding_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new()
        .single_child(|child| Padding::new(EdgeInsets::all(8.0)).child(child));
}

#[test]
fn padding_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| Padding::new(EdgeInsets::all(8.0)).child(child));
}
