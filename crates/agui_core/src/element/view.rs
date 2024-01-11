use crate::{element::render::ElementRender, render::binding::ViewBinding};

pub trait ElementView: ElementRender {
    /// Creates a binding for this element's view.
    fn create_binding(&mut self) -> Box<dyn ViewBinding>;
}
