use crate::{unit::Constraints, widget::Widget};

use super::lifecycle::ElementLifecycle;

pub mod erased;
pub mod resolver;

/// A deferred element is an element that is not immediately build its children,
/// but instead waits until the layout phase to do so.
///
/// It creates a resolver that is passed into the layout phase, which is used to
/// determine if the element needs to be rebuilt. If the resolver returns a value
/// that is not equal to the previous value, the element will be rebuilt.
///
/// Resolver functions should be cheap to call, and should _only_ return a different
/// value if it is absolutely necessary, as it will stall the rendering phase in
/// order to rebuild the element leading to a poor user experience.
pub trait ElementDeferred: ElementLifecycle {
    type Param: PartialEq + Send;

    fn create_resolver(&self) -> impl Fn(Constraints) -> Self::Param + Send + 'static;

    fn build(&self, param: &Self::Param) -> Widget;
}
