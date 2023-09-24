pub mod canvas;
pub(crate) mod manager;
pub mod paint;
pub mod renderer;
pub mod texture;

pub use paint::*;
pub use texture::*;

pub type CanvasPainter<'paint> = canvas::painter::CanvasPainter<'paint, canvas::painter::Head<()>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RenderViewId(Option<usize>);

impl RenderViewId {
    pub(crate) fn new(id: usize) -> Self {
        Self(Some(id))
    }
}
