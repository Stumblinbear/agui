mod instance;

use agui_core::{
    render::CanvasPainter,
    widget::{AnyWidget, Widget},
};
pub use instance::*;

pub trait WidgetPaint: AnyWidget {
    fn child(&self) -> Option<Widget> {
        None
    }

    /// Called whenever this widget is redrawn.
    #[allow(unused_variables)]
    fn paint(&self, canvas: CanvasPainter);
}
