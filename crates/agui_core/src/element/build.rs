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

impl std::fmt::Debug for Box<dyn ElementBuild> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct((**self).short_type_name())
            .finish_non_exhaustive()
    }
}
