use std::{ops::Deref, rc::Rc};

use agui_core::listeners::EventEmitter;

use crate::event::WinitWindowEvent;

#[derive(Clone)]
pub struct WinitWindowHandle {
    handle: Rc<winit::window::Window>,
    event_emitter: EventEmitter<WinitWindowEvent>,
}

impl WinitWindowHandle {
    pub fn new(window: winit::window::Window) -> Self {
        Self {
            handle: Rc::new(window),
            event_emitter: EventEmitter::default(),
        }
    }

    pub fn events(&self) -> &EventEmitter<WinitWindowEvent> {
        &self.event_emitter
    }
}

impl Deref for WinitWindowHandle {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        self.handle.as_ref()
    }
}
