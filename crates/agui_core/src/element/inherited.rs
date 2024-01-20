use std::any::TypeId;

use crate::widget::Widget;

use super::widget::ElementWidget;

pub trait ElementInherited: ElementWidget {
    /// This must return the type id of the widget that this element is for.
    fn inherited_type_id(&self) -> TypeId;

    fn child(&self) -> Widget;

    /// Called during the build phase to determine if elements listening to
    /// this element need to be rebuilt.
    fn needs_notify(&mut self) -> bool;
}
