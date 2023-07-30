mod context;
mod instance;

pub use context::*;
pub use instance::*;

use super::IntoChild;

pub trait StatefulWidget: Sized + 'static {
    type State: WidgetState<Widget = Self>;

    fn create_state(&self) -> Self::State;
}

pub trait WidgetState: Sized + 'static {
    type Widget;

    type Child: IntoChild;

    /// Called when the widget is replaced in the tree by a new widget of the same concrete type.
    #[allow(unused_variables)]
    fn updated(&mut self, new_widget: &Self::Widget) {}

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Self::Child;
}
