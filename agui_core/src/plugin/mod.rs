use downcast_rs::{impl_downcast, Downcast};

use crate::WidgetManager;

use self::event::WidgetEvent;

pub mod event;

pub trait WidgetPlugin: Downcast {
    fn on_event(&mut self, manager: &mut WidgetManager, event: &WidgetEvent);
}

impl_downcast!(WidgetPlugin);