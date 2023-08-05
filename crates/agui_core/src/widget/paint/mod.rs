use crate::render::CanvasPainter;

mod instance;

pub use instance::*;

pub trait WidgetPaint: Sized + 'static {
    /// Called whenever this widget is redrawn.
    #[allow(unused_variables)]
    fn paint(&self, canvas: CanvasPainter);
}
