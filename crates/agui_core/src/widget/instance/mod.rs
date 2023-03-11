use std::rc::Rc;

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{Constraints, Data, IntrinsicDimension, Size},
    widget::WidgetRef,
};

use super::AnyWidget;

mod context;
mod inherited;
mod stateful;
mod stateless;

pub use context::*;
pub use inherited::InheritedInstance;
pub use stateful::StatefulInstance;
pub use stateless::StatelessInstance;

pub trait ElementWidget {
    fn type_name(&self) -> &'static str;

    fn is_similar(&self, other: &WidgetRef) -> bool;

    fn get_widget(&self) -> Rc<dyn AnyWidget>;

    fn intrinsic_size(
        &self,
        ctx: WidgetIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32;

    fn layout(&self, ctx: WidgetLayoutContext, constraints: Constraints) -> Size;

    fn build(&mut self, ctx: WidgetBuildContext) -> Vec<WidgetRef>;

    fn update(&mut self, old: WidgetRef) -> bool;

    fn paint(&self, size: Size) -> Option<Canvas>;

    #[allow(clippy::borrowed_box)]
    fn call(
        &mut self,
        ctx: WidgetCallbackContext,
        callback_id: CallbackId,
        arg: &Box<dyn Data>,
    ) -> bool;
}

impl std::fmt::Debug for Box<dyn ElementWidget> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.type_name()).finish_non_exhaustive()
    }
}
