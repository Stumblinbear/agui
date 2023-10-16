use std::ops::Deref;

use agui_core::listenable::Event;
use winit::event::WindowEvent;

pub struct WinitWindowEvent(pub WindowEvent<'static>);

impl Event for WinitWindowEvent {}

impl Deref for WinitWindowEvent {
    type Target = WindowEvent<'static>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<WindowEvent<'static>> for WinitWindowEvent {
    fn from(event: WindowEvent<'static>) -> Self {
        Self(event)
    }
}
