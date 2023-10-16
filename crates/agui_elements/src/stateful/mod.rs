mod context;
mod instance;

use agui_core::widget::Widget;
pub use context::*;
pub use instance::*;

pub trait StatefulWidget: 'static {
    type State: WidgetState<Widget = Self>;

    fn create_state(&self) -> Self::State;
}

pub trait WidgetState: 'static {
    type Widget;

    /// Called when the widget is first added to the tree.
    #[allow(unused_variables)]
    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {}

    /// Called when the widget was replaced in the tree by a new widget of the same concrete type.
    #[allow(unused_variables)]
    fn updated(&mut self, ctx: &mut StatefulBuildContext<Self>, old_widget: &Self::Widget) {}

    /// Called whenever this widget is rebuilt.
    ///
    /// This method may be called when any parent is rebuilt or when its internal state changes.
    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget;
}
