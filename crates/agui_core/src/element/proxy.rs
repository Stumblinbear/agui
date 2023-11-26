use crate::widget::Widget;

use super::widget::ElementWidget;

pub trait ElementProxy: ElementWidget {
    fn child(&self) -> Widget;
}

impl std::fmt::Debug for Box<dyn ElementProxy> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct((**self).short_type_name())
            .finish_non_exhaustive()
    }
}
