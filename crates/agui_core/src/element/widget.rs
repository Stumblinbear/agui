use crate::{unit::AsAny, widget::Widget};

use super::{ElementMountContext, ElementUnmountContext, ElementUpdate};

pub trait ElementWidget: AsAny {
    #[allow(unused_variables)]
    fn mount(&mut self, ctx: &mut ElementMountContext) {}

    #[allow(unused_variables)]
    fn unmount(&mut self, ctx: &mut ElementUnmountContext) {}

    /// Returns true if the widget is of the same type as the other widget.
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;
}

impl std::fmt::Debug for Box<dyn ElementWidget> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct((**self).short_type_name())
            .finish_non_exhaustive()
    }
}
