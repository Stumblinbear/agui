use crate::{element::render::ElementRender, engine::rendering::view::View};

pub trait ElementView: ElementRender {
    /// Creates a view for this subtree.
    fn create_view(&mut self) -> Box<dyn View + Send>;
}
