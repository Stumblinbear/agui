mod context;
mod instance;

pub use context::*;
pub use instance::*;

use super::Widget;

pub trait WidgetBuild: Sized + 'static {
    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&self, ctx: &mut BuildContext<Self>) -> Widget;
}
