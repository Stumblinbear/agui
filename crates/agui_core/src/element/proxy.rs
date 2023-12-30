use crate::widget::Widget;

use super::widget::ElementWidget;

pub trait ElementProxy: ElementWidget {
    fn child(&self) -> Widget;
}
