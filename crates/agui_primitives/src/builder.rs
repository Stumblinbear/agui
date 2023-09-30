use agui_core::widget::{BuildContext, Widget, WidgetBuild};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget)]
pub struct Builder {
    #[allow(clippy::type_complexity)]
    pub func: Box<dyn Fn(&mut BuildContext<Builder>) -> Widget>,
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
    fn build(&self, ctx: &mut BuildContext<Self>) -> Widget {
        (self.func)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        engine::Engine,
        query::WidgetQueryExt,
        unit::{Constraints, Size},
        widget::{BuildContext, LayoutContext, Widget, WidgetLayout},
    };
    use agui_macros::LayoutWidget;

    use crate::builder::Builder;

    #[derive(LayoutWidget, Debug, Default, PartialEq)]
    struct TestWidget {}

    impl WidgetLayout for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
            vec![]
        }

        fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
            Size::ZERO
        }
    }

    #[test]
    pub fn calls_func() {
        let mut engine = Engine::builder()
            .with_root(Builder::new(|_| TestWidget::default().into()))
            .build();

        engine.update();

        assert!(
            engine.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
