use std::sync::mpsc::Sender;

use agui_core::{
    callback::Callback,
    element::ElementId,
    widget::{InheritedWidget, Widget},
};
use agui_macros::InheritedWidget;
use winit::window::WindowBuilder;

use crate::handle::WinitWindowHandle;

#[derive(InheritedWidget)]
pub struct WinitWindowingController {
    pub tx: Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,

    pub child: Widget,
}

impl InheritedWidget for WinitWindowingController {
    fn get_child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

impl WinitWindowingController {
    pub fn create_window(
        &self,
        window_element_id: ElementId,
        builder: WindowBuilder,
        callback: Callback<WinitWindowHandle>,
    ) {
        self.tx
            .send((window_element_id, builder, callback))
            .unwrap();
    }
}
