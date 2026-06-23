use std::any::Any;
use std::rc::Rc;

use agui_core::tree::NodeHandle;

use crate::provide::ProvideScope;

/// The context passed to a widget's `build`: the values in scope, but no cursor and no pipeline. A build
/// composes a child widget description; it cannot restructure the tree or plant render boundaries.
pub struct BuildCtx {
    provide: ProvideScope,
    handle: NodeHandle,
}

impl BuildCtx {
    pub(crate) fn new(provide: ProvideScope, handle: NodeHandle) -> Self {
        Self { provide, handle }
    }

    /// The nearest provided value of type `T` in scope, or `None`, without recording a dependency.
    pub fn get_provided<T: Any>(&self) -> Option<Rc<T>> {
        self.provide.get::<T>()
    }

    /// The nearest provided value of type `T`, recording this element as a dependent so a later change to
    /// that value reruns its `dependency_changed`.
    pub fn depend_on_provided<T: Any>(&self) -> Option<Rc<T>> {
        self.provide.get_and_depend::<T>(self.handle)
    }
}
