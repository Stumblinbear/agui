use std::{ops::Deref, rc::Rc};

use agui_core::notifier::{ListenerHandle, Notifier};

use crate::event::WindowEvent;

#[derive(Clone)]
pub struct WinitWindowHandle(Rc<InnerHandle>);

struct InnerHandle {
    handle: winit::window::Window,

    notifier: Notifier<WindowEvent>,
}

impl WinitWindowHandle {
    pub(crate) fn new(window: winit::window::Window) -> Self {
        Self(Rc::new(InnerHandle {
            handle: window,

            notifier: Notifier::new(),
        }))
    }

    pub fn notify(&self, event: &WindowEvent) {
        self.0.notifier.notify(event);
    }

    pub fn add_listener(
        &self,
        func: impl Fn(&WindowEvent) + 'static,
    ) -> ListenerHandle<WindowEvent> {
        self.0.notifier.add_listener(func)
    }
}

impl PartialEq for WinitWindowHandle {
    fn eq(&self, other: &Self) -> bool {
        // war crimes
        std::ptr::eq(
            Rc::as_ptr(&self.0) as *const _ as *const (),
            Rc::as_ptr(&other.0) as *const _ as *const (),
        )
    }
}

impl Deref for WinitWindowHandle {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        &self.0.handle
    }
}
