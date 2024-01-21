use std::rc::Rc;

use crate::{element::lifecycle::ElementLifecycle, widget::AnyWidget};

pub trait ElementWidget: ElementLifecycle {
    type Widget: AnyWidget
    where
        Self: Sized;

    fn widget(&self) -> &Rc<Self::Widget>
    where
        Self: Sized;
}
