use std::any::Any;

use crate::{callback::CallbackId, element::ElementUpdate, unit::AsAny};

use super::widget::Widget;

mod context;

pub use context::*;

pub trait ElementWidget: AsAny {
    fn widget_name(&self) -> &'static str;

    #[allow(unused_variables)]
    fn mount(&mut self, ctx: WidgetMountContext) {}

    #[allow(unused_variables)]
    fn unmount(&mut self, ctx: WidgetUnmountContext) {}

    /// Returns true if the widget is of the same type as the other widget.
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;
}

impl std::fmt::Debug for Box<dyn ElementWidget> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}

pub trait ElementBuild: ElementWidget {
    fn build(&mut self, ctx: WidgetBuildContext) -> Widget;

    #[allow(unused_variables)]
    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool;
}

impl std::fmt::Debug for Box<dyn ElementBuild> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}
