use agui_core::widget::{BuildContext, Widget, WidgetBuild};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget)]
pub struct Builder {
    #[allow(clippy::type_complexity)]
    pub func: Box<dyn Fn(&mut BuildContext<Self>) -> Widget>,
}

impl Builder {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut BuildContext<Self>) -> Widget + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }
}

impl WidgetBuild for Builder {
    type Child = Widget;

    fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
        (self.func)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        manager::WidgetManager,
        query::WidgetQueryExt,
        widget::{BuildContext, WidgetBuild},
    };
    use agui_macros::StatelessWidget;

    use crate::Builder;

    #[derive(StatelessWidget, Debug, Default, PartialEq)]
    struct TestWidget {}

    impl WidgetBuild for TestWidget {
        type Child = ();

        fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {}
    }

    #[test]
    pub fn calls_func() {
        let mut manager = WidgetManager::with_root(Builder::new(|_| TestWidget::default().into()));

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
