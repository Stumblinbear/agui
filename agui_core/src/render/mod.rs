use std::any::TypeId;

use crate::{widget::WidgetID, WidgetManager};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct WidgetChanged {
    pub type_id: TypeId,
    pub widget_id: WidgetID,
}

pub trait WidgetRenderer {
    fn added(&mut self, manager: &WidgetManager, changed: WidgetChanged);

    fn refresh(&mut self, manager: &WidgetManager, changed: WidgetChanged);

    fn removed(&mut self, manager: &WidgetManager, changed: WidgetChanged);
}
