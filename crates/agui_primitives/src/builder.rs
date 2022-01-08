use agui_core::{
    context::WidgetContext,
    widget::{BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

#[derive(Widget)]
#[widget(into = false)]
pub struct Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    func: F,
}

impl<F> std::fmt::Debug for Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Builder").finish()
    }
}

impl<F> Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> WidgetBuilder for Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        (self.func)(ctx)
    }
}

impl<F> From<Builder<F>> for WidgetRef
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn from(builder: Builder<F>) -> Self {
        Self::new(builder)
    }
}
