use std::ops::Deref;

use winit::event::WindowEvent;

#[derive(Clone, Debug)]
pub struct WinitWindowEvent(pub WindowEvent<'static>);

impl AsRef<WindowEvent<'static>> for WinitWindowEvent {
    fn as_ref(&self) -> &WindowEvent<'static> {
        &self.0
    }
}

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
