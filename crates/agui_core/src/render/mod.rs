pub mod binding;
pub mod canvas;
mod object;
mod paint;

pub use object::*;
pub use paint::*;

pub type CanvasPainter<'paint> = canvas::painter::CanvasPainter<'paint, canvas::painter::Head<()>>;
