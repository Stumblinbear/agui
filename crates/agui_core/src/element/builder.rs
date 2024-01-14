use std::rc::Rc;

use crate::element::ElementType;

pub trait ElementBuilder: 'static {
    fn create_element(self: Rc<Self>) -> ElementType;
}
