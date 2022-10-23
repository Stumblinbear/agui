pub mod canvas;
pub mod paint;
pub mod texture;

pub use paint::*;
pub use texture::*;

pub type CanvasPainter<'paint> = canvas::painter::CanvasPainter<'paint, canvas::painter::Head>;
