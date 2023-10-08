use agui_core::widget::Widget;
use agui_elements::stateless::{StatelessBuildContext, StatelessWidget};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget)]
pub struct Builder {
    #[allow(clippy::type_complexity)]
    pub func: Box<dyn Fn(&mut StatelessBuildContext<Builder>) -> Widget>,
}

impl Builder {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut StatelessBuildContext<Self>) -> Widget + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }
}

impl StatelessWidget for Builder {
    fn build(&self, ctx: &mut StatelessBuildContext<Self>) -> Widget {
        (self.func)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        engine::Engine,
        query::WidgetQueryExt,
        unit::{Constraints, IntrinsicDimension, Size},
        widget::Widget,
    };
    use agui_elements::layout::{IntrinsicSizeContext, LayoutContext, WidgetLayout};
    use agui_macros::LayoutWidget;

    use crate::builder::Builder;

    #[derive(LayoutWidget, Debug, Default, PartialEq)]
    struct TestWidget {}

    impl WidgetLayout for TestWidget {
        fn get_children(&self) -> Vec<Widget> {
            vec![]
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
