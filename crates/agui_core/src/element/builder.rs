use std::rc::Rc;

use crate::element::{lifecycle::ElementLifecycle, Element};

pub trait ElementBuilder {
    type Element: ElementLifecycle;

    fn create_element(self: Rc<Self>) -> Element;
}
