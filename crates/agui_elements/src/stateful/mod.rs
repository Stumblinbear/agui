use agui_core::widget::Widget;

mod context;
mod element;

pub use context::*;
pub use element::*;

pub trait StatefulWidget: Sized {
    type State: WidgetState<Widget = Self>;

    fn create_state(&self) -> Self::State;
}

pub trait WidgetState {
    type Widget;

    /// Called when the widget is first added to the tree.
    #[allow(unused_variables)]
    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {}

    /// Called when the widget was replaced in the tree by a new widget of the same concrete type.
    #[allow(unused_variables)]
    fn updated(&mut self, ctx: &mut StatefulBuildContext<Self>, old_widget: &Self::Widget) {}

    /// Called whenever this widget is rebuilt or when its state changes.
    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget;
}
