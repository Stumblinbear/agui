pub mod by_element;
pub mod by_key;
pub mod by_widget;

#[cfg(test)]
mod tests {
    use crate::{
        element::mock::{build::MockBuildWidget, render::MockRenderWidget},
        engine::elements::{strategies::mocks::MockInflateElements, ElementTree},
        query::{
            by_key::FilterKeyExt,
            by_widget::{ExactWidgetIterator, FilterByWidgetExt},
        },
        unit::Key,
        widget::{IntoWidget, Widget},
    };

    #[test]
    pub fn finds_widget_by_key_and_downcasts() {
        let root_widget = MockRenderWidget::default();
        {
            root_widget.mock().expect_children().returning(|| {
                let build_widget = MockBuildWidget::default();
                {
                    build_widget
                        .mock
                        .borrow_mut()
                        .expect_build()
                        .returning(|_| {
                            let build_widget = MockBuildWidget::default();
                            {
                                build_widget
                                    .mock
                                    .borrow_mut()
                                    .expect_build()
                                    .returning(|_| MockRenderWidget::dummy());
                            }
                            build_widget.into_widget()
                        });
                }
                vec![
                    Widget::new_with_key(Key::local(0), build_widget.clone()),
                    Widget::new_with_key(Key::local(1), build_widget.clone()),
                    Widget::new_with_key(Key::local(2), build_widget),
                ]
            });
        }

        let mut tree = ElementTree::default();

        tree.inflate(
            &mut MockInflateElements::default(),
            root_widget.into_widget(),
        )
        .expect("failed to spawn and inflate");

        assert_eq!(
            tree.iter()
                .filter_key(Key::local(1))
                .filter_widget::<MockBuildWidget>()
                .and_downcast()
                .count(),
            1,
            "should have found and downcasted 1 widget with local key 1"
        );
    }
}
