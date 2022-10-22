use std::ops::Deref;

use crate::widget::{Widget, WidgetState};

pub struct RenderContext<'ctx, W>
where
    W: Widget + WidgetState,
{
    pub widget: &'ctx W,
    pub state: &'ctx W::State,
}

impl<W> Deref for RenderContext<'_, W>
where
    W: Widget + WidgetState,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> RenderContext<'_, W>
where
    W: Widget + WidgetState,
{
    pub fn get_widget(&self) -> &W {
        self.widget
    }
}

impl<W> RenderContext<'_, W>
where
    W: Widget + WidgetState,
{
    pub fn get_state(&self) -> &W::State {
        self.state
    }
}
