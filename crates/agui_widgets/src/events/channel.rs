use std::sync::mpsc;

use agui_core::widget::{InheritedWidget, Widget};
use agui_macros::InheritedWidget;

#[derive(Debug, InheritedWidget)]
pub struct EventChannel<Event: 'static> {
    pub receiver: mpsc::Receiver<Event>,

    #[child]
    pub child: Option<Widget>,
}

impl<Event> InheritedWidget for EventChannel<Event> {
    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}
