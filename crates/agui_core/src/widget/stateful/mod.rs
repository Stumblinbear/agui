mod context;
mod instance;

pub use context::*;
pub use instance::*;

use super::Widget;

pub trait StatefulWidget: Sized + 'static {
    type State: WidgetState<Widget = Self>;

    fn create_state(&self) -> Self::State;
}

pub trait WidgetState: Sized + 'static {
    type Widget;

    /// Called when the widget is first added to the tree.
    #[allow(unused_variables)]
    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {}

    /// Called when the widget is replaced in the tree by a new widget of the same concrete type.
    #[allow(unused_variables)]
    fn updated(&mut self, new_widget: &Self::Widget) {}

    /// Called when any of the widget's dependencies have changed.
    #[allow(unused_variables)]
    fn dependencies_changed(&mut self, ctx: &mut StatefulBuildContext<Self>) {}

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget;
}
