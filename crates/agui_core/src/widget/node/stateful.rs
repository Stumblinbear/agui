use downcast_rs::Downcast;

use crate::{
    manager::{widget::WidgetBuilder, Data},
    widget::{BuildContext, BuildResult},
};

/// Implements the widget's `build()` method.
pub trait StatefulWidget: std::fmt::Debug + Downcast + Sized {
    type State: Data + Default;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt, when its internal state changes, or
    /// just because it feels like it.
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult;
}

impl<W> WidgetBuilder for W
where
    W: StatefulWidget,
{
    type State = W::State;

    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        self.build(ctx)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        manager::{context::Context, query::WidgetQueryExt, WidgetManager},
        widget::{BuildContext, BuildResult},
    };

    use super::StatefulWidget;

    #[derive(Debug, Default, Clone, Copy)]
    struct TestGlobal(i32);

    #[derive(Debug, Default)]
    struct TestWidget {}

    impl StatefulWidget for TestWidget {
        type State = u64;

        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            ctx.set_state(|state| {
                *state += 1;
            });

            BuildResult::None
        }
    }

    #[test]
    pub fn widget_build_can_set_state() {
        let mut manager = WidgetManager::with_root(TestWidget::default());

        manager.update();

        assert_eq!(
            *manager
                .query()
                .by_type::<TestWidget>()
                .next()
                .unwrap()
                .get_state(),
            1,
            "widget `u32` should be 1"
        );

        manager.update();

        assert_eq!(
            *manager
                .query()
                .by_type::<TestWidget>()
                .next()
                .unwrap()
                .get_state(),
            1,
            "widget `u32` should still be 1"
        );
    }
}
