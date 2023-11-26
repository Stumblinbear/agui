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
    use agui_core::{element::mock::DummyWidget, engine::Engine, query::WidgetQueryExt};

    use crate::builder::Builder;

    #[test]
    pub fn calls_func() {
        let mut engine = Engine::builder()
            .with_root(Builder::new(|_| DummyWidget.into()))
            .build();

        engine.update();

        assert!(
            engine.query().by_type::<DummyWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
