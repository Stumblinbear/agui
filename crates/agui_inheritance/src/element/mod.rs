use agui_core::widget::{AnyWidget, Widget};

mod instance;
#[cfg(any(test, feature = "mocks"))]
pub mod mock;

pub use instance::*;

pub trait InheritedWidget: AnyWidget {
    fn get_child(&self) -> Widget;

    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool;
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use agui_core::{
        element::mock::{build::MockBuildWidget, render::MockRenderWidget, DummyWidget},
        engine::Engine,
        unit::Size,
        widget::{IntoWidget, Widget},
    };
    use agui_macros::InheritedWidget;

    use crate::{context::ContextInheritedMut, InheritancePlugin};

    use super::InheritedWidget;

    #[derive(InheritedWidget)]
    struct TestInheritedWidget {
        data: usize,

        child: Widget,
    }

    impl InheritedWidget for TestInheritedWidget {
        fn get_child(&self) -> Widget {
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
        fn get_child(&self) -> Widget {
            self.child.clone()
        }

        fn should_notify(&self, _: &Self) -> bool {
            true
        }
    }

    // TODO: add more test cases

    #[test]
    pub fn updates_scoped_children() {
        let (root_widget, root_children) = create_widget("RootWidget");

        let (depending_widget, inherited_data) = create_depending_widget("DependingWidget");

        let mut engine = Engine::builder()
            .add_plugin(InheritancePlugin::default())
            .with_root(root_widget)
            .build();

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        }
        .into_widget()];

        engine.update();

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

        engine.mark_dirty(engine.get_root());

        engine.update();

        assert_eq!(
            *inherited_data.borrow(),
            Some(9),
            "should have updated the child widget"
        );
    }

    #[test]
    pub fn updates_nested_scope_children() {
        let (root_widget, root_children) = create_widget("RootWidget");

        let (depending_widget, inherited_data) = create_depending_widget("DependingWidget");

        let nested_scope = TestOtherInheritedWidget {
            child: depending_widget,
        }
        .into_widget();

        *root_children.borrow_mut() = vec![DummyWidget.into_widget()];

        let mut engine = Engine::builder()
            .add_plugin(InheritancePlugin::default())
            .with_root(root_widget)
            .build();

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: nested_scope.clone(),
        }
        .into_widget()];

        engine.update();

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

        engine.mark_dirty(engine.get_root());

        engine.update();

        assert_eq!(
            *inherited_data.borrow(),
            Some(9),
            "should have updated the child widget"
        );
    }

    #[test]
    pub fn child_updates_when_dependency_unavailable() {
        let (root_widget, root_children) = create_widget("RootWidget");

        let (depending_widget, inherited_data) = create_depending_widget("DependingWidget");

        let mut engine = Engine::builder()
            .add_plugin(InheritancePlugin::default())
            .with_root(root_widget)
            .build();

        *root_children.borrow_mut() = vec![TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        }
        .into_widget()];

        engine.update();

        assert_eq!(
            *inherited_data.borrow(),
            Some(7),
            "should have retrieved the inherited widget"
        );

        *root_children.borrow_mut() = vec![depending_widget.clone()];

        engine.mark_dirty(engine.get_root());

        engine.update();

        assert_eq!(
            *inherited_data.borrow(),
            None,
            "should have updated the child widget"
        );
    }

    fn create_widget(name: &'static str) -> (Widget, Rc<RefCell<Vec<Widget>>>) {
        let children = Rc::new(RefCell::new(Vec::new()));

        let widget = MockRenderWidget::new(name);
        {
            widget
                .mock
                .borrow_mut()
                .expect_get_children()
                .returning_st({
                    let children = children.clone();

                    move || children.borrow().clone()
                });

            widget
                .mock
                .borrow_mut()
                .expect_layout()
                .returning(|_, _| Size::ZERO);
        }
        let widget = widget.into_widget();

        (widget, children)
    }

    fn create_depending_widget(name: &'static str) -> (Widget, Rc<RefCell<Option<usize>>>) {
        let inherited_data = Rc::new(RefCell::new(None));

        let depending_widget = MockBuildWidget::new(name);
        {
            depending_widget
                .mock
                .borrow_mut()
                .expect_build()
                .returning_st({
                    let inherited_data = inherited_data.clone();

                    move |mut ctx| {
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
