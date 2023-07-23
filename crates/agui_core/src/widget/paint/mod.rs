use crate::render::CanvasPainter;

mod context;
mod instance;

pub use context::*;
pub use instance::*;

pub trait WidgetPaint: Sized + 'static {
    /// Called whenever this widget is redrawn.
    #[allow(unused_variables)]
    fn paint(&self, canvas: CanvasPainter);
}
