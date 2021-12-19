use agui_core::{unit::Layout, BuildResult, WidgetContext, WidgetImpl, WidgetRef};
use agui_macros::Widget;

#[derive(Widget)]
pub struct Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    func: F,
}

impl<F> Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> WidgetImpl for Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn layout(&self) -> Option<&Layout> {
        None
    }

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

impl<F> From<Builder<F>> for Option<WidgetRef>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn from(builder: Builder<F>) -> Self {
        Some(WidgetRef::new(builder))
    }
}
