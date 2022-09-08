use std::any::TypeId;

use downcast_rs::{impl_downcast, Downcast};

use crate::{
    callback::CallbackId,
    manager::context::AguiContext,
    render::canvas::painter::CanvasPainter,
    unit::{Data, Layout, LayoutType, Rect},
};

use super::{BuildResult, WidgetKey};

pub trait WidgetInstance: Downcast {
    fn get_type_id(&self) -> TypeId;
    fn get_display_name(&self) -> String;

    fn get_key(&self) -> Option<WidgetKey>;
    fn set_key(&mut self, key: WidgetKey);

    fn get_layout_type(&self) -> Option<LayoutType>;
    fn get_layout(&self) -> Option<Layout>;

    fn set_rect(&mut self, rect: Option<Rect>);
    fn get_rect(&self) -> Option<Rect>;

    fn build(&mut self, ctx: AguiContext) -> BuildResult;

    fn get_canvas(&self) -> Option<&Canvas>;
    fn render(&mut self, rect: Rect);

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool;
}

impl_downcast!(WidgetInstance);
