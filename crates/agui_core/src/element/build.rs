use std::any::Any;

use crate::{callback::CallbackId, widget::Widget};

use super::{widget::ElementWidget, ElementBuildContext, ElementCallbackContext};

pub trait ElementBuild: ElementWidget {
    fn build(&mut self, ctx: &mut ElementBuildContext) -> Widget;

    #[allow(unused_variables)]
    fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool;
}
