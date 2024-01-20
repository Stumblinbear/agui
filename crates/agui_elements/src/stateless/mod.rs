use agui_core::widget::Widget;

mod context;
mod element;

pub use context::*;
pub use element::*;

pub trait StatelessWidget: Sized {
    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut StatelessBuildContext<Self>) -> Widget;
}
