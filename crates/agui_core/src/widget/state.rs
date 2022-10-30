use crate::unit::Data;

pub trait WidgetState {
    type State: Data;

    fn create_state(&self) -> Self::State;

    /// Called when the widget is replaced in the tree by a new widget of the same concrete type.
    ///
    /// If the return value is `true`, the widget will be rebuilt, otherwise it will be kept as is.
    #[allow(unused_variables)]
    fn updated(&self, other: &Self) -> bool {
        true
    }
}
