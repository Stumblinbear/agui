use crate::widget::{element::ElementWidget, Widget};

pub trait ElementProxy: ElementWidget {
    fn get_child(&self) -> Widget;
}

impl std::fmt::Debug for Box<dyn ElementProxy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(self.widget_name()).finish_non_exhaustive()
    }
}
