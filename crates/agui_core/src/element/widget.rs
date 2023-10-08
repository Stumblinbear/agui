use crate::{unit::AsAny, widget::Widget};

use super::{ElementMountContext, ElementUnmountContext, ElementUpdate};

pub trait ElementWidget: AsAny {
    fn widget_name(&self) -> &'static str;

    #[allow(unused_variables)]
    fn mount(&mut self, ctx: ElementMountContext) {}

    #[allow(unused_variables)]
    fn unmount(&mut self, ctx: ElementUnmountContext) {}

    /// Returns true if the widget is of the same type as the other widget.
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;
}

impl std::fmt::Debug for Box<dyn ElementWidget> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}
