use downcast_rs::{impl_downcast, Downcast};

use crate::{
    callback::CallbackId,
    manager::context::AguiContext,
    render::canvas::Canvas,
    unit::{Data, Rect},
};

use super::{BuildResult, LayoutResult, WidgetRef};

pub enum WidgetEquality {
    /// Indicates that the two widgets are exactly equal.
    ///
    /// The engine will immediately stop rebuilding the tree starting from this widget, as
    /// it can guarantee that it, nor its children, have changed.
    Equal,

    /// Indicates that the two widgets are of equal types, but their parameters differ.
    ///
    /// The engine will retain the state of the widget, but will rebuild the widget and continue
    /// rebuilding the tree.
    Unequal,

    /// Indicates that the two widgets are of different types.
    ///
    /// The engine will destroy the widget completely and continue rebuilding the tree.
    Invalid,
}

pub trait WidgetDispatch: Downcast {
    fn is_similar(&self, other: &WidgetRef) -> WidgetEquality;

    fn update(&mut self, other: WidgetRef) -> bool;

    fn layout(&mut self, ctx: AguiContext) -> LayoutResult;

    fn build(&mut self, ctx: AguiContext) -> BuildResult;

    fn render(&self, rect: Rect) -> Option<Canvas>;

    #[allow(clippy::borrowed_box)]
    fn call(&mut self, ctx: AguiContext, callback_id: CallbackId, arg: &Box<dyn Data>) -> bool;
}

impl_downcast!(WidgetDispatch);
