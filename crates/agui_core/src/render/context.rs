use std::ops::Deref;

use crate::widget::WidgetImpl;

pub struct RenderContext<'ctx, W>
where
    W: WidgetImpl,
{
    pub widget: &'ctx W,
    pub state: &'ctx W::State,
}

impl<W> Deref for RenderContext<'_, W>
where
    W: WidgetImpl,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> RenderContext<'_, W>
where
    W: WidgetImpl,
{
    pub fn get_widget(&self) -> &W {
        self.widget
    }

    pub fn get_state(&self) -> &W::State {
        self.state
    }
}
