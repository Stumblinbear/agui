use agui::widgets::flex::Column;
use agui_test::ElementLifecycleCheck;

#[test]
fn column_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new()
        .multi_child(|children| Column::builder().children(children).build());
}
