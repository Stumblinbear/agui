mod instance;

use std::rc::Rc;

pub use instance::*;

use super::{AnyWidget, Widget};

pub trait InheritedWidget: AnyWidget {
    fn get_child(&self) -> Widget;

    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool;
}

pub trait ContextInheritedMut {
    fn depend_on_inherited_widget<I>(&mut self) -> Option<Rc<I>>
    where
        I: AnyWidget + InheritedWidget;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::{InheritedWidget, LayoutWidget, StatelessWidget};

    use crate::{
        engine::Engine,
        unit::{Constraints, IntrinsicDimension, Size},
        widget::{
            BuildContext, InheritedWidget, IntoWidget, IntrinsicSizeContext, LayoutContext, Widget,
            WidgetBuild, WidgetLayout,
        },
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

    #[derive(Default, LayoutWidget)]
    struct TestRootWidget;

    impl WidgetLayout for TestRootWidget {
        fn get_children(&self) -> Vec<Widget> {
            Vec::from_iter(TEST_HOOK.with(|result| result.borrow().root_child.clone()))
        }

        fn intrinsic_size(
            &self,
            _: &mut IntrinsicSizeContext,
            _: IntrinsicDimension,
            _: f32,
        ) -> f32 {
            0.0
        }

        fn layout(
            &self,
            _: &mut crate::widget::LayoutContext,
            _: crate::unit::Constraints,
        ) -> Size {
            Size::ZERO
        }
    }

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

    #[derive(Default, LayoutWidget)]
    struct TestDummyWidget {
        pub child: Option<Widget>,
    }

    impl WidgetLayout for TestDummyWidget {
        fn get_children(&self) -> Vec<Widget> {
            self.child.clone().into_iter().collect()
        }

        fn intrinsic_size(
            &self,
            _: &mut IntrinsicSizeContext,
            _: IntrinsicDimension,
            _: f32,
        ) -> f32 {
            0.0
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    #[derive(StatelessWidget, Default)]
    struct TestDependingWidget;

    impl WidgetBuild for TestDependingWidget {
        fn build(&self, ctx: &mut BuildContext<Self>) -> Widget {
            let widget = ctx.depend_on_inherited_widget::<TestInheritedWidget>();

            TEST_HOOK.with(|result| {
                result.borrow_mut().inherited_data = widget.map(|w| w.data);
            });

            TestDummyWidget { child: None }.into_widget()
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
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        let depending_widget = TestDependingWidget.into_widget();

        set_root_child(TestInheritedWidget {
            data: 7,
            child: depending_widget.clone(),
        });

        engine.update();

        assert_inherited_data(Some(7), "should have retrieved the inherited widget");

        set_root_child(TestInheritedWidget {
            data: 9,
            child: depending_widget.clone(),
        });

        engine.mark_dirty(engine.get_root().unwrap());
        engine.update();

        assert_inherited_data(Some(9), "should have updated the child widget");
    }

    #[test]
    pub fn updates_nested_scope_children() {
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        let nested_scope = TestOtherInheritedWidget {
            child: TestDependingWidget.into(),
        }
        .into_widget();

        set_root_child(TestInheritedWidget {
            data: 7,
            child: nested_scope.clone(),
        });

        engine.update();

        assert_inherited_data(Some(7), "should have retrieved the inherited widget");

        set_root_child(TestInheritedWidget {
            data: 9,
            child: nested_scope,
        });

        engine.mark_dirty(engine.get_root().unwrap());
        engine.update();

        assert_inherited_data(Some(9), "should have updated the child widget");
    }

    #[test]
    pub fn child_updates_when_dependency_unavailable() {
        let mut engine = Engine::builder().with_root(TestRootWidget).build();

        let dependent_child = TestDependingWidget.into_widget();

        set_root_child(TestInheritedWidget {
            data: 7,
            child: dependent_child.clone(),
        });

        engine.update();

        assert_inherited_data(Some(7), "should have retrieved the inherited widget");

        set_root_child(TestDummyWidget {
            child: dependent_child.into(),
        });

        engine.mark_dirty(engine.get_root().unwrap());
        engine.update();

        assert_inherited_data(None, "should have updated the child widget");
    }
}
