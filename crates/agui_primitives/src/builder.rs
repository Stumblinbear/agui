use agui_core::widget::{BuildContext, BuildResult, WidgetBuilder};

pub struct Builder {
    func: Box<dyn Fn(&mut BuildContext<Self>) -> BuildResult + 'static>,
}

impl std::fmt::Debug for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder").finish()
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
