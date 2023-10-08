use std::rc::Rc;

use super::ElementType;

pub trait ElementBuilder: 'static {
    fn create_element(self: Rc<Self>) -> ElementType;
}
