use crate::render::CanvasPainter;

mod instance;

pub use instance::*;

use super::{AnyWidget, Widget};

pub trait WidgetPaint: AnyWidget {
    fn get_child(&self) -> Option<Widget> {
        None
    }

    /// Called whenever this widget is redrawn.
    #[allow(unused_variables)]
    fn paint(&self, canvas: CanvasPainter);
}
