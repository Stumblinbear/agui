use agui_core::widget::Widget;

mod element;

pub use element::*;

pub trait InheritedWidget: Sized {
    fn child(&self) -> Widget;

    fn should_notify(&self, old_widget: &Self) -> bool;
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use agui_core::{
        element::{
            mock::{
                build::MockBuildWidget,
                render::{MockRenderObject, MockRenderWidget},
            },
            ElementComparison,
        },
        engine::elements::{strategies::mocks::MockInflateElementStrategy, ElementTree},
        widget::{IntoWidget, Widget},
    };
    use agui_macros::InheritedWidget;

    use super::InheritedWidget;

    #[derive(InheritedWidget)]
    struct TestInheritedWidget {
        data: usize,

        child: Widget,
    }

    impl InheritedWidget for TestInheritedWidget {
        fn child(&self) -> Widget {
            self.child.clone()
        }

        fn should_notify(&self, _: &Self) -> bool {
            true
        }
    }

    #[derive(InheritedWidget)]
    struct TestOtherInheritedWidget {
        child: Widget,
    }

    impl InheritedWidget for TestOtherInheritedWidget {
        fn child(&self) -> Widget {
            self.child.clone()
        }

        fn should_notify(&self, _: &Self) -> bool {
            true
        }
    }

    // TODO: add more test cases

    #[test]
    pub fn updates_scoped_children() {
        let (root_widget, root_children) = create_widget();

        let (depending_widget, inherited_data) = create_depending_widget();

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        }
        .into_widget()];

        let mut tree = ElementTree::new();

        let root_id = tree
            .inflate(
                &mut MockInflateElementStrategy::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        assert_eq!(
            *inherited_data.borrow(),
            Some(7),
            "should have retrieved the inherited widget"
        );

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 9,
            child: depending_widget.clone(),
        }
        .into_widget()];

        tree.rebuild(&mut MockInflateElementStrategy::default(), root_id)
            .expect("failed to rebuild");

        assert_eq!(
            *inherited_data.borrow(),
            Some(9),
            "should have updated the child widget"
        );
    }

    #[test]
    pub fn updates_nested_scope_children() {
        let (root_widget, root_children) = create_widget();

        let (depending_widget, inherited_data) = create_depending_widget();

        let nested_scope = TestOtherInheritedWidget {
            child: depending_widget,
        }
        .into_widget();

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: nested_scope.clone(),
        }
        .into_widget()];

        let mut tree = ElementTree::new();

        let root_id = tree
            .inflate(
                &mut MockInflateElementStrategy::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        assert_eq!(
            *inherited_data.borrow(),
            Some(7),
            "should have retrieved the inherited widget"
        );

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 9,
            child: nested_scope.clone(),
        }
        .into_widget()];

        tree.rebuild(&mut MockInflateElementStrategy::default(), root_id)
            .expect("failed to rebuild");

        assert_eq!(
            *inherited_data.borrow(),
            Some(9),
            "should have updated the child widget"
        );
    }

    #[test]
    pub fn child_updates_when_dependency_unavailable() {
        let (root_widget, root_children) = create_widget();

        let (depending_widget, inherited_data) = create_depending_widget();

        let mut tree = ElementTree::new();

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        }
        .into_widget()];

        let root_id = tree
            .inflate(
                &mut MockInflateElementStrategy::default(),
                root_widget.into_widget(),
            )
            .expect("failed to inflate widget");

        assert_eq!(
            *inherited_data.borrow(),
            Some(7),
            "should have retrieved the inherited widget"
        );

        *root_children.borrow_mut() = vec![depending_widget.clone()];

        tree.rebuild(&mut MockInflateElementStrategy::default(), root_id)
            .expect("failed to rebuild");

        assert_eq!(
            *inherited_data.borrow(),
            None,
            "should have updated the child widget"
        );
    }

    fn create_widget() -> (Widget, Rc<RefCell<Vec<Widget>>>) {
        let children = Rc::new(RefCell::new(Vec::new()));

        let widget = MockRenderWidget::default();
        {
            let mut widget_mock = widget.mock();

            widget_mock.expect_children().returning_st({
                let children = children.clone();

                move || children.borrow().clone()
            });

            widget_mock.expect_update().returning(|new_widget| {
                if new_widget.downcast::<MockRenderWidget>().is_some() {
                    ElementComparison::Changed
                } else {
                    ElementComparison::Invalid
                }
            });

            widget_mock
                .expect_create_render_object()
                .returning(|_| MockRenderObject::dummy());

            widget_mock
                .expect_update_render_object()
                .returning(|_, _| {});
        }
        let widget = widget.into_widget();

        (widget, children)
    }

    fn create_depending_widget() -> (Widget, Rc<RefCell<Option<usize>>>) {
        let inherited_data = Rc::new(RefCell::new(None));

        let depending_widget = MockBuildWidget::default();
        {
            depending_widget
                .mock
                .borrow_mut()
                .expect_build()
                .returning_st({
                    let inherited_data = inherited_data.clone();

                    move |ctx| {
                        let widget = ctx.depend_on_inherited_widget::<TestInheritedWidget>();

                        *inherited_data.borrow_mut() = widget.map(|widget| widget.data);

                        MockRenderWidget::dummy()
                    }
                });

            depending_widget
                .mock
                .borrow_mut()
                .expect_update()
                .returning_st(move |_| ElementComparison::Identical);
        }
        let depending_widget = depending_widget.into_widget();

        (depending_widget, inherited_data)
    }
}
