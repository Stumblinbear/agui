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
                build::MockBuildWidget, render::MockRenderWidget, DummyRenderObject, DummyWidget,
            },
            ElementComparison,
        },
        engine::widgets::WidgetManager,
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

        let mut manager = WidgetManager::default_with_root(root_widget.into_widget());

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        }
        .into_widget()];

        manager.update();

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

        manager.mark_needs_build(manager.root().expect("no root element"));

        manager.update();

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

        *root_children.borrow_mut() = vec![DummyWidget.into_widget()];

        let mut manager = WidgetManager::default_with_root(root_widget.into_widget());

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: nested_scope.clone(),
        }
        .into_widget()];

        manager.update();

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

        manager.mark_needs_build(manager.root().expect("no root element"));

        manager.update();

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

        let mut manager = WidgetManager::default_with_root(root_widget.into_widget());

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        }
        .into_widget()];

        manager.update();

        assert_eq!(
            *inherited_data.borrow(),
            Some(7),
            "should have retrieved the inherited widget"
        );

        *root_children.borrow_mut() = vec![depending_widget.clone()];

        manager.mark_needs_build(manager.root().expect("no root element"));

        manager.update();

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
            let mut widget_mock = widget.mock.borrow_mut();

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
                .returning(|_| DummyRenderObject.into());

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

                        DummyWidget.into_widget()
                    }
                });
        }
        let depending_widget = depending_widget.into_widget();

        (depending_widget, inherited_data)
    }
}
