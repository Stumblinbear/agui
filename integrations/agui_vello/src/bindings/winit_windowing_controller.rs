use std::sync::mpsc::Sender;

use agui::{element::ElementId, prelude::*};
use winit::window::WindowBuilder;

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

#[derive(PartialEq)]
pub struct WinitWindowHandle {
    pub window_id: winit::window::WindowId,

    pub title: String,
}
