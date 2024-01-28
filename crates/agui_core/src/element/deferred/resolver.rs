use std::any::Any;

use crate::unit::Constraints;

pub(crate) trait DeferredResolver: Any + Send {
    fn resolve(&mut self, constraints: Constraints) -> bool;

    fn param(&self) -> Option<&(dyn Any + Send)>;
}
