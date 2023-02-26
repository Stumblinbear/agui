use downcast_rs::Downcast;

use crate::{
    callback::CallbackId,
    element::context::ElementContext,
    render::canvas::Canvas,
    unit::{Data, Size},
    widget::{Children, LayoutResult, WidgetRef},
};

use super::AnyWidget;

mod stateful;
mod stateless;

pub use stateful::StatefulInstance;
pub use stateless::StatelessInstance;

pub trait ElementWidget: Downcast {
    fn type_name(&self) -> &'static str;

    fn is_similar(&self, other: &WidgetRef) -> bool;

    fn update(&mut self, other: WidgetRef) -> bool;

    fn layout(&mut self, ctx: ElementContext) -> LayoutResult;

    fn build(&mut self, ctx: ElementContext) -> Children;

    fn paint(&self, size: Size) -> Option<Canvas>;

    #[allow(clippy::borrowed_box)]
    fn call(&mut self, ctx: ElementContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool;
}

impl std::fmt::Debug for Box<dyn ElementWidget> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.type_name()).finish_non_exhaustive()
    }
}
