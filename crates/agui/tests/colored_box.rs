use agui::{paint::peniko::Color, widgets::colored_box::ColoredBox};
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn colored_box_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| ColoredBox::new(Color::BLACK).child(child));
}

#[test]
fn colored_box_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| ColoredBox::new(Color::BLACK).child(child));
}
