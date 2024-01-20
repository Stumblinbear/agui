use std::rc::Rc;

use crate::element::ElementType;

pub trait ElementBuilder {
    fn create_element(self: Rc<Self>) -> ElementType;
}
