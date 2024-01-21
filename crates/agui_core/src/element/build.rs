use std::any::Any;

use crate::{callback::CallbackId, widget::Widget};

use super::{lifecycle::ElementLifecycle, ElementBuildContext, ElementCallbackContext};

pub trait ElementBuild: ElementLifecycle {
    fn build(&mut self, ctx: &mut ElementBuildContext) -> Widget;

    fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool;
}
