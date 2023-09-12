use std::sync::mpsc::Sender;

use agui_core::{
    callback::Callback,
    element::ElementId,
    widget::{InheritedWidget, IntoWidget, Widget},
};
use agui_macros::InheritedWidget;
use agui_primitives::sized_box::SizedBox;
use winit::window::WindowBuilder;

use crate::handle::WinitWindowHandle;

#[derive(InheritedWidget)]
pub struct WinitWindowingController {
    pub tx: Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,

    pub child: Option<Widget>,
}

impl InheritedWidget for WinitWindowingController {
    fn get_child(&self) -> Widget {
        self.child
            .clone()
            .unwrap_or_else(|| SizedBox::shrink().into_widget())
    }

    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

impl WinitWindowingController {
    pub const fn new(tx: Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>) -> Self {
        Self { tx, child: None }
    }

    pub fn with_child<T: IntoWidget>(mut self, child: impl Into<Option<T>>) -> Self {
        self.child = child.into().map(IntoWidget::into_widget);

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
