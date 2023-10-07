use std::any::TypeId;

use crate::widget::{element::ElementWidget, Widget};

pub trait ElementInherited: ElementWidget {
    fn get_child(&self) -> Widget;

    fn should_notify(&mut self) -> bool;
}

impl std::fmt::Debug for Box<dyn ElementInherited> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}
