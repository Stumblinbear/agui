use std::sync::mpsc;

use agui_core::widget::{InheritedWidget, Widget};
use agui_macros::InheritedWidget;

#[derive(InheritedWidget)]
pub struct EventChannel<Event: 'static> {
    pub receiver: mpsc::Receiver<Event>,

    pub child: Widget,
}

impl<Event> InheritedWidget for EventChannel<Event> {
    fn get_child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}
