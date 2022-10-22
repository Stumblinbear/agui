use std::rc::Rc;

use downcast_rs::{impl_downcast, Downcast};

use crate::unit::Data;

use super::{dispatch::WidgetDispatch, BuildContext, BuildResult, LayoutContext, LayoutResult};

pub trait Widget: Downcast {
    fn into_widget(self: Rc<Self>) -> Box<dyn WidgetDispatch>;
}

impl_downcast!(Widget);

pub trait WidgetState: Widget + PartialEq {
    type State: Data;

    fn create_state(&self) -> Self::State;

    /// Called when the widget is replaced in the tree by a new widget of the same concrete type.
    ///
    /// If the return value is `true`, the widget will be rebuilt, otherwise it will be kept as is.
    #[allow(unused_variables)]
    fn updated(&self, other: &Self) -> bool {
        true
    }
}

/// Implements the widget's `layout()` and `build()` method.
pub trait WidgetView: Widget + WidgetState + Sized {
    #[allow(unused_variables)]
    fn layout(&self, ctx: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult::default()
    }

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use agui_macros::StatefulWidget;

    use crate::{
        manager::WidgetManager,
        widget::{BuildContext, BuildResult, ContextStatefulWidget},
    };

    use super::{WidgetState, WidgetView};

    thread_local! {
        pub static STATE: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(Debug, Default, StatefulWidget, PartialEq)]
    struct TestWidget {}

    impl WidgetState for TestWidget {
        type State = u32;

        fn create_state(&self) -> Self::State {
            0
        }
    }

    impl WidgetView for TestWidget {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            ctx.set_state(|state| {
                *state += 1;
            });

            STATE.with(|f| {
                f.borrow_mut().push(*ctx.state);
            });

            BuildResult::empty()
        }
    }

    #[test]
    pub fn widget_build_can_set_state() {
        let mut manager = WidgetManager::with_root(TestWidget::default());

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[0], 1, "widget `u32` should be 1");
        });

        manager.update();

        STATE.with(|f| {
            assert_eq!(
                f.borrow().len(),
                1,
                "widget `u32` should not have been updated"
            );
        });

        manager.mark_dirty(manager.get_root().unwrap());

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[1], 2, "widget `u32` should have updated to 2");
        });
    }
}
