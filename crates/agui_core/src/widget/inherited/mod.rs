mod instance;

use std::rc::Rc;

pub use instance::*;

use super::{AnyWidget, WidgetChild};

pub trait InheritedWidget: WidgetChild {
    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        true
    }
}

pub trait ContextInheritedMut {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::{InheritedWidget, StatelessWidget};

    use crate::{
        manager::WidgetManager,
        widget::{BuildContext, InheritedWidget, IntoChild, IntoWidget, Widget, WidgetBuild},
    };

    use super::ContextInheritedMut;

    #[derive(Default)]
    struct TestResult {
        root_child: Option<Widget>,

        inherited_data: Option<usize>,
    }

    thread_local! {
        static TEST_HOOK: RefCell<TestResult> = RefCell::default();
    }

    #[derive(Default, StatelessWidget)]
    struct TestRootWidget;

    impl WidgetBuild for TestRootWidget {
        type Child = Option<Widget>;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            TEST_HOOK.with(|result| result.borrow().root_child.clone())
        }
    }

    #[derive(Default, InheritedWidget)]
    struct TestInheritedWidget {
        data: usize,

        #[child]
        pub child: Option<Widget>,
    }

    impl InheritedWidget for TestInheritedWidget {}

    #[derive(Default, InheritedWidget)]
    struct TestOtherInheritedWidget {
        #[child]
        pub child: Option<Widget>,
    }

    impl InheritedWidget for TestOtherInheritedWidget {}

    #[derive(Default, StatelessWidget)]
    struct TestDummyWidget {
        pub child: Option<Widget>,
    }

    impl WidgetBuild for TestDummyWidget {
        type Child = Option<Widget>;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            self.child.clone().into_child()
        }
    }

    #[derive(StatelessWidget, Default)]
    struct TestDependingWidget;

    impl WidgetBuild for TestDependingWidget {
        type Child = ();

        fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
            let widget = ctx.depend_on_inherited_widget::<TestInheritedWidget>();

            TEST_HOOK.with(|result| {
                result.borrow_mut().inherited_data = widget.map(|w| w.data);
            });
        }
    }

    fn set_root_child(child: impl IntoWidget) {
        TEST_HOOK.with(|result| {
            result.borrow_mut().root_child = Some(child.into_widget());
        });
    }

    fn assert_inherited_data(data: Option<usize>, message: &'static str) {
        TEST_HOOK.with(|result| {
            assert_eq!(result.borrow().inherited_data, data, "{}", message);
        });
    }

    // TODO: add more test cases

    #[test]
    pub fn updates_scoped_children() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestRootWidget);

        let depending_widget = TestDependingWidget.into_widget();

        set_root_child(TestInheritedWidget {
            data: 7,
            child: depending_widget.clone().into(),
        });

        manager.update();

        assert_inherited_data(Some(7), "should have retrieved the inherited widget");

        set_root_child(TestInheritedWidget {
            data: 9,
            child: depending_widget.into(),
        });

        manager.mark_dirty(manager.get_root().unwrap());
        manager.update();

        assert_inherited_data(Some(9), "should have updated the child widget");
    }

    #[test]
    pub fn updates_nested_scope_children() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestRootWidget);

        let nested_scope = TestOtherInheritedWidget {
            child: TestDependingWidget.into(),
        }
        .into_widget();

        set_root_child(TestInheritedWidget {
            data: 7,
            child: nested_scope.clone().into(),
        });

        manager.update();

        assert_inherited_data(Some(7), "should have retrieved the inherited widget");

        set_root_child(TestInheritedWidget {
            data: 9,
            child: nested_scope.into(),
        });

        manager.mark_dirty(manager.get_root().unwrap());
        manager.update();

        assert_inherited_data(Some(9), "should have updated the child widget");
    }

    #[test]
    pub fn child_updates_when_dependency_unavailable() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestRootWidget);

        let dependent_child = TestDependingWidget.into_widget();

        set_root_child(TestInheritedWidget {
            data: 7,
            child: dependent_child.clone().into(),
        });

        manager.update();

        assert_inherited_data(Some(7), "should have retrieved the inherited widget");

        set_root_child(TestDummyWidget {
            child: dependent_child.into(),
        });

        manager.mark_dirty(manager.get_root().unwrap());
        manager.update();

        assert_inherited_data(None, "should have updated the child widget");
    }
}
