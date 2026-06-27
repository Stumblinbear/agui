use agui::widgets::opacity::Opacity;
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn opacity_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new().single_child(|child| Opacity::new(0.5).child(child));
}

#[test]
fn opacity_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| Opacity::new(0.5).child(child));
}
