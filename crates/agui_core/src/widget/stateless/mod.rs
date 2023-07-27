mod context;
mod instance;

pub use context::*;
pub use instance::*;

use super::IntoChild;

pub trait WidgetChild: Sized + 'static {
    type Child: IntoChild;

    /// Called whenever this widget is first.
    fn get_child(&self) -> Self::Child;
}

pub trait WidgetBuild: Sized + 'static {
    type Child: IntoChild;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child;
}
