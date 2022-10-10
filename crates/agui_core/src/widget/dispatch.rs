use downcast_rs::{impl_downcast, Downcast};

use crate::{
    callback::CallbackId,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Rect},
};

use super::{BuildResult, WidgetRef};

pub trait WidgetDispatch: Downcast {
    fn is_similar(&self, other: &WidgetRef) -> bool;

    fn build(&mut self, ctx: AguiContext) -> BuildResult;

    fn render(&self, rect: Rect) -> Option<Canvas>;

    #[allow(clippy::borrowed_box)]
    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool;
}

impl_downcast!(WidgetDispatch);
