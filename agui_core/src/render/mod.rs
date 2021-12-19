pub mod color;

use crate::{WidgetID, WidgetManager};

pub trait WidgetRenderer {
    fn create(&mut self, manager: &WidgetManager, widget_id: WidgetID);

    fn refresh(&mut self, manager: &WidgetManager);

    fn remove(&mut self, manager: &WidgetManager, widget_id: WidgetID);
}
