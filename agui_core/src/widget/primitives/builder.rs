use std::any::TypeId;

use crate::{
    widget::{BuildResult, Layout, Widget},
    WidgetContext,
};

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
    pub fn new(func: F) -> Builder<F> {
        Builder { func }
    }
}

impl<F> Widget for Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }

    fn layout(&self) -> Option<&Layout> {
        None
    }

    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        (self.func)(ctx)
    }
}

impl<F> Into<Box<dyn Widget>> for Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn into(self) -> Box<dyn Widget> {
        Box::new(self)
    }
}

impl<F> Into<Option<Box<dyn Widget>>> for Builder<F>
where
    F: Fn(&WidgetContext) -> BuildResult + 'static,
{
    fn into(self) -> Option<Box<dyn Widget>> {
        Some(Box::new(self))
    }
}
