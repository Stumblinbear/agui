use crate::{unit::AsAny, widget::Widget};

use super::{ElementComparison, ElementMountContext, ElementUnmountContext};

pub trait ElementLifecycle: AsAny {
    #[allow(unused_variables)]
    fn mount(&mut self, ctx: &mut ElementMountContext) {}

    #[allow(unused_variables)]
    fn unmount(&mut self, ctx: &mut ElementUnmountContext) {}

    fn update(&mut self, new_widget: &Widget) -> ElementComparison;
}
