use downcast_rs::{impl_downcast, Downcast};

use crate::{
    callback::CallbackId,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Layout, LayoutType, Rect},
};

use super::{BuildResult, WidgetRef};

pub trait WidgetInstance: Downcast {
    fn is_similar(&self, other: &WidgetRef) -> bool;

    fn get_layout_type(&self) -> Option<LayoutType>;
    fn get_layout(&self) -> Option<Layout>;

    fn set_rect(&mut self, rect: Option<Rect>);
    fn get_rect(&self) -> Option<Rect>;

    fn build(&mut self, ctx: AguiContext) -> BuildResult;

    fn render(&self) -> Option<Canvas>;

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool;
}

impl_downcast!(WidgetInstance);
