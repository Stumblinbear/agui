use agui::{paint::peniko::kurbo::Affine, widgets::transform::Transform};
use agui_test::{ElementLifecycleCheck, sizing::BoxSizingCheck};

#[test]
fn transform_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new()
        .single_child(|child| Transform::new(Affine::IDENTITY).child(child));
}

#[test]
fn transform_obeys_box_sizing() {
    BoxSizingCheck::default().single_child(|child| Transform::new(Affine::IDENTITY).child(child));
}
