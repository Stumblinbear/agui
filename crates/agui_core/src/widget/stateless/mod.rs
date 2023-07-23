mod context;
mod instance;

pub use context::*;
pub use instance::*;

use super::IntoChildren;

pub trait WidgetChild: Sized + 'static {
    #[cfg(not(nightly))]
    type Child: IntoChildren;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    #[cfg(not(nightly))]
    fn get_child(&self) -> Self::Child;

    #[cfg(nightly)]
    fn get_child(&self) -> impl IntoChildren;
}

pub trait WidgetBuild: Sized + 'static {
    #[cfg(not(nightly))]
    type Child: IntoChildren;

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    #[cfg(not(nightly))]
    fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child;

    #[cfg(nightly)]
    fn build(&self, ctx: &mut BuildContext<Self>) -> impl IntoChildren;
}
