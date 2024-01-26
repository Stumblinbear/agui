use crate::{unit::Constraints, widget::Widget};

use super::{lifecycle::ElementLifecycle, ElementBuildContext};

pub trait ElementDeferred: ElementLifecycle {
    fn build(&mut self, ctx: &mut ElementBuildContext, constraints: Constraints) -> Widget;
}
