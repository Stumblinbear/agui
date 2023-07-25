mod context;
mod instance;

pub use context::*;
pub use instance::*;

use super::IntoChildren;

pub trait StatefulWidget: Sized + 'static {
    type State: WidgetState<Widget = Self>;

    fn create_state(&self) -> Self::State;
}

pub trait WidgetState: Sized + 'static {
    type Widget;

    #[cfg(not(nightly))]
    type Child: IntoChildren;

    /// Called when the widget is replaced in the tree by a new widget of the same concrete type.
    #[allow(unused_variables)]
    fn updated(&self, new_widget: &Self::Widget) {}

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    #[cfg(not(nightly))]
    fn build(&self, ctx: &mut StatefulBuildContext<Self>) -> Self::Child;

    #[cfg(nightly)]
    fn build(&self, ctx: &mut StatefulBuildContext<Self>) -> impl IntoChildren;
}
