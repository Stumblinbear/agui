use std::ops::Deref;

use crate::{
    unit::Data,
    widget::{WidgetState, WidgetView},
};

pub struct PaintContext<'ctx, W>
where
    W: WidgetView,
{
    pub widget: &'ctx W,
    pub(crate) state: &'ctx dyn Data,
}

impl<W> Deref for PaintContext<'_, W>
where
    W: WidgetView + WidgetState,
{
    type Target = W::State;

    fn deref(&self) -> &Self::Target {
        self.state.downcast_ref().unwrap()
    }
}
