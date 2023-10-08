use std::sync::mpsc;

use agui_core::{callback::Callback, element::ElementId, plugin::Plugin};
use winit::window::WindowBuilder;

use crate::WinitWindowHandle;

pub struct WinitPlugin {
    tx: mpsc::Sender<WinitBindingEvent>,
}

impl WinitPlugin {
    pub fn new(tx: mpsc::Sender<WinitBindingEvent>) -> Self {
        Self { tx }
    }
}

impl Plugin for WinitPlugin {}

impl WinitPlugin {
    pub(crate) fn create_window(
        &self,
        window_element_id: ElementId,
        window: WindowBuilder,
        callback: Callback<WinitWindowHandle>,
    ) {
        let _ = self.tx.send(WinitBindingEvent::CreateWindow(
            window_element_id,
            Box::new(window),
            callback,
        ));
    }
}

pub enum WinitBindingEvent {
    CreateWindow(ElementId, Box<WindowBuilder>, Callback<WinitWindowHandle>),
    CloseWindow(ElementId),
}
