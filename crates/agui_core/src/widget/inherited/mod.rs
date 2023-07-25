mod inheritance;
mod instance;

pub(crate) use inheritance::*;
pub use instance::*;

use super::{AnyWidget, WidgetChild};

pub trait InheritedWidget: WidgetChild {
    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        true
    }
}

pub trait ContextInheritedMut {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<&I>
    where
        I: AnyWidget + InheritedWidget;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::{InheritedWidget, StatelessWidget};

    use crate::{
        manager::WidgetManager,
        widget::{BuildContext, InheritedWidget, IntoWidget, WidgetBuild, WidgetRef},
    };

    use super::ContextInheritedMut;

    #[derive(Default)]
    struct TestResult {
        root_child: WidgetRef,

        inherited_data: Option<usize>,
    }

    thread_local! {
        static TEST_HOOK: RefCell<TestResult> = RefCell::default();
    }

    #[derive(Default, StatelessWidget)]
    struct TestRootWidget;

    impl WidgetBuild for TestRootWidget {
        type Child = WidgetRef;

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
            TEST_HOOK.with(|result| result.borrow().root_child.clone())
        }
    }

    #[derive(Default, InheritedWidget)]
    struct TestInheritedWidget {
        data: usize,

        #[child]
        pub child: WidgetRef,
    }

    impl InheritedWidget for TestInheritedWidget {}

    #[derive(StatelessWidget, Default)]
    struct TestDependingWidget;

    impl WidgetBuild for TestDependingWidget {
        type Child = WidgetRef;

        fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
            let widget = ctx.depend_on_inherited_widget::<TestInheritedWidget>();

            TEST_HOOK.with(|result| {
                result.borrow_mut().inherited_data = widget.map(|w| w.data);
            });

            WidgetRef::None
        }
    }

    fn set_inherited_data(data: usize, child: WidgetRef) {
        TEST_HOOK.with(|result| {
            result.borrow_mut().root_child = TestInheritedWidget { data, child }.into_widget();
        });
    }

    fn assert_inherited_data(data: usize, message: &'static str) {
        TEST_HOOK.with(|result| {
            assert_eq!(result.borrow().inherited_data, Some(data), "{}", message);
        });
    }

    #[test]
    pub fn can_reference_inherited_widget() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestRootWidget);

        set_inherited_data(7, TestDependingWidget.into_widget());
        manager.update();
        assert_inherited_data(7, "should have retrieved the inherited widget");
    }

    #[test]
    pub fn can_receive_updates() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestRootWidget);

        let depending_widget = TestDependingWidget.into_widget();

        set_inherited_data(4, depending_widget.clone());
        manager.update();
        assert_inherited_data(4, "should have retrieved the inherited widget");

        set_inherited_data(9, depending_widget);
        manager.mark_dirty(manager.get_root().unwrap());
        manager.update();
        assert_inherited_data(9, "should have updated the child widget");
    }
}
