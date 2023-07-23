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
        widget::{BuildContext, InheritedWidget, WidgetBuild, WidgetRef},
    };

    use super::ContextInheritedMut;

    #[derive(Default)]
    struct TestResult {
        data: Option<usize>,
    }

    thread_local! {
        static TEST_RESULT: RefCell<TestResult> = RefCell::default();
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

            TEST_RESULT.with(|result| {
                result.borrow_mut().data = widget.map(|w| w.data);
            });

            WidgetRef::None
        }
    }

    #[test]
    pub fn can_reference_inherited_widget() {
        let mut manager = WidgetManager::new();

        manager.set_root(TestInheritedWidget {
            data: 7,

            child: TestDependingWidget.into(),
        });

        manager.update();

        TEST_RESULT.with(|result| {
            assert_eq!(
                result.borrow().data,
                Some(7),
                "should have retrieved the inherited widget"
            );
        });
    }
}
