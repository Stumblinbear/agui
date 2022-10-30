use std::ops::Deref;

use crate::widget::Widget;

pub struct PaintContext<'ctx, W>
where
    W: Widget,
{
    pub widget: &'ctx W,
    pub state: &'ctx W::State,
}

impl<W> Deref for PaintContext<'_, W>
where
    W: Widget,
{
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.widget
    }
}

impl<W> PaintContext<'_, W>
where
    W: Widget,
{
    pub fn get_widget(&self) -> &W {
        self.widget
    }

    pub fn get_state(&self) -> &W::State {
        self.state
    }
}
