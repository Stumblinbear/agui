use std::rc::Rc;

use crate::{element::render::ElementRender, render::ViewBinding};

pub trait ElementView: ElementRender {
    /// Returns the binding for this element's view.
    fn binding(&self) -> &Rc<dyn ViewBinding>;
}
