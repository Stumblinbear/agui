use crate::{unit::AsAny, widget::Widget};

use super::{ElementComparison, ElementMountContext, ElementUnmountContext};

pub trait ElementWidget: AsAny {
    #[allow(unused_variables)]
    fn mount(&mut self, ctx: &mut ElementMountContext) {}

    #[allow(unused_variables)]
    fn unmount(&mut self, ctx: &mut ElementUnmountContext) {}

    /// Returns true if the widget is of the same type as the other widget.
    fn update(&mut self, new_widget: &Widget) -> ElementComparison;
}
