use std::sync::mpsc;

use agui_core::{callback::Callback, element::ElementId, widget::Widget};
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;
use winit::window::WindowBuilder;

use crate::handle::WinitWindowHandle;

#[derive(InheritedWidget)]
pub struct WinitBinding {
    tx: mpsc::Sender<WinitBindingEvent>,

    #[prop(into)]
    child: Widget,
}

impl InheritedWidget for WinitBinding {
    fn get_child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

impl WinitBinding {
    pub fn create_window(
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
