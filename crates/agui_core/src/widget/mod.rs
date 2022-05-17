use std::any::TypeId;

use downcast_rs::{impl_downcast, Downcast};

use crate::{
    callback::CallbackId,
    canvas::{Canvas, Root},
    manager::{context::AguiContext, widget::WidgetId, Data},
    unit::{Layout, LayoutType, Rect},
};

mod context;
mod key;
mod node;
mod result;

pub use self::{
    context::*,
    key::*,
    node::{StatefulWidget, StatelessWidget},
    result::BuildResult,
};

pub trait WidgetImpl: std::fmt::Debug + Downcast {
    fn get_type_id(&self) -> TypeId;
    fn get_display_name(&self) -> String;

    fn get_layout_type(&self) -> Option<LayoutType>;
    fn get_layout(&self) -> Option<Layout>;

    fn set_rect(&mut self, rect: Option<Rect>);
    fn get_rect(&self) -> Option<Rect>;

    fn build(&mut self, ctx: AguiContext) -> BuildResult;

    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &dyn Data) -> bool;

    fn render(&self, canvas: &mut Canvas<Root>);
}

impl_downcast!(WidgetImpl);

/// Implements the widget's `build()` method.
pub trait WidgetBuilder: std::fmt::Debug + Downcast + Sized {
    type State: Data + Default;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult;
}
