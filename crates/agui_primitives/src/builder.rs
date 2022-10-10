use agui_core::widget::{BuildContext, BuildResult, WidgetBuilder};

pub struct Builder {
    #[allow(clippy::type_complexity)]
    pub func: Box<dyn Fn(&mut BuildContext<Self>) -> BuildResult>,
}

impl PartialEq for Builder {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

impl Builder {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut BuildContext<Self>) -> BuildResult + 'static,
    {
        Self {
            func: Box::new(func),
        }
    }
}

impl WidgetBuilder for Builder {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        (self.func)(ctx)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{
        manager::WidgetManager,
        query::WidgetQueryExt,
        widget::{BuildContext, BuildResult, WidgetBuilder},
    };

    use crate::Builder;

    #[derive(Debug, Default, PartialEq)]
    struct TestWidget {}

    impl WidgetBuilder for TestWidget {
        fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
            BuildResult::empty()
        }
    }

    #[test]
    pub fn calls_func() {
        let mut manager = WidgetManager::with_root(Builder::new(|_| {
            BuildResult::with_children([TestWidget::default()])
        }));

        manager.update();

        assert!(
            manager.query().by_type::<TestWidget>().next().is_some(),
            "widget should have been created"
        );
    }
}
