use std::sync::mpsc;

use agui_core::widget::{InheritedWidget, WidgetRef};
use agui_macros::InheritedWidget;

#[derive(Debug, InheritedWidget)]
pub struct EventChannel<Event: 'static> {
    pub receiver: mpsc::Receiver<Event>,

    #[child]
    pub child: WidgetRef,
}

impl<Event> InheritedWidget for EventChannel<Event> {}
