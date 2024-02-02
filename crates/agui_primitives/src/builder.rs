use agui_core::widget::Widget;
use agui_elements::stateless::{StatelessBuildContext, StatelessWidget};
use agui_macros::StatelessWidget;

// TODO: replace this with an custom element instead of a stateless widget

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
        element::mock::render::MockRenderWidget,
        engine::elements::{strategies::mocks::MockInflateElementStrategy, ElementTree},
        query::by_widget::FilterByWidgetExt,
        widget::IntoWidget,
    };

    use crate::builder::Builder;

    #[test]
    pub fn calls_func() {
        let mut tree = ElementTree::new();

        tree.inflate(
            &mut MockInflateElementStrategy::default(),
            None,
            Builder::new(|_| MockRenderWidget::dummy()).into_widget(),
        )
        .expect("failed to inflate widget");

        assert_eq!(
            tree.iter().filter_widget::<MockRenderWidget>().count(),
            1,
            "widget should have been created"
        );
    }
}
