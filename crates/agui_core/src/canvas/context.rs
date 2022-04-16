use std::ops::Deref;

use crate::engine::widget::WidgetBuilder;

pub struct RenderContext<'ctx, W>
where
    W: WidgetBuilder,
{
    pub(crate) widget: &'ctx W,
    pub(crate) state: &'ctx W::State,
}

impl<W> Deref for RenderContext<'_, W>
where
    W: WidgetBuilder,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> RenderContext<'_, W>
where
    W: WidgetBuilder,
{
    pub fn get_state(&self) -> &W::State {
        self.state
    }
}
