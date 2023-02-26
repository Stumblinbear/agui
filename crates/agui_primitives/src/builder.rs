use agui_core::widget::{BuildContext, Children, WidgetView};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget)]
pub struct Builder {
    #[allow(clippy::type_complexity)]
    pub func: Box<dyn Fn(&mut BuildContext<Self>) -> Children>,
}

impl PartialEq for Builder {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Builder {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut BuildContext<Self>) -> Children + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }
}

impl WidgetView for Builder {
    fn build(&self, ctx: &mut BuildContext<Self>) -> Children {
        (self.func)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        manager::WidgetManager,
        query::WidgetQueryExt,
        widget::{BuildContext, Children, WidgetView},
    };
    use agui_macros::StatelessWidget;

    use crate::Builder;

    #[derive(StatelessWidget, Debug, Default, PartialEq)]
    struct TestWidget {}

    impl WidgetView for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> Children {
            Children::none()
        }
    }

    #[test]
    pub fn calls_func() {
        let mut manager =
            WidgetManager::with_root(Builder::new(|_| Children::from([TestWidget::default()])));

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
