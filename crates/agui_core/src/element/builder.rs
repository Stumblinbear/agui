use std::rc::Rc;

use crate::element::{lifecycle::ElementLifecycle, ElementType};

pub trait ElementBuilder {
    type Element: ElementLifecycle;

    fn create_element(self: Rc<Self>) -> ElementType;
}
