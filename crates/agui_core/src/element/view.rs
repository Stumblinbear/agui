use crate::{element::render::ElementRender, render::view::View};

pub trait ElementView: ElementRender {
    /// Creates a the view for this subtree.
    fn create_view(&self) -> Box<dyn View + Send>;
}
