use std::sync::mpsc::Sender;

use agui_core::{
    callback::Callback,
    element::ElementId,
    widget::{InheritedWidget, IntoChild, Widget},
};
use agui_macros::InheritedWidget;
use winit::window::WindowBuilder;

use crate::handle::WinitWindowHandle;

#[derive(InheritedWidget)]
pub struct WinitWindowingController {
    pub tx: Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,

    #[child]
    pub child: Option<Widget>,
}

impl InheritedWidget for WinitWindowingController {
    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

impl WinitWindowingController {
    pub fn new(tx: Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>) -> Self {
        Self { tx, child: None }
    }

    pub fn with_child(mut self, child: impl IntoChild) -> Self {
        self.child = child.into_child();

        self
    }

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
