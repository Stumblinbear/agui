use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct WinitWindowHandle(Arc<winit::window::Window>);

impl WinitWindowHandle {
    pub(crate) fn new(handle: winit::window::Window) -> Self {
        Self(Arc::new(handle))
    }
}

impl Deref for WinitWindowHandle {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
