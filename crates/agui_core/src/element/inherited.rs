use crate::widget::Widget;

use super::widget::ElementWidget;

pub trait ElementInherited: ElementWidget {
    fn child(&self) -> Widget;

    /// Called during the build phase to determine if elements listening to
    /// this element need to be rebuilt.
    fn needs_notify(&mut self) -> bool;
}
