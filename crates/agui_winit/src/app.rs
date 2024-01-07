use agui_core::callback::Callback;
use rustc_hash::FxHashMap;
use winit::{
    event::Event as WinitEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{WindowBuilder, WindowId},
};

use crate::WinitWindowHandle;

pub struct WinitApp {
    pub event_loop: EventLoop<WinitBindingAction>,

    window_renderer: FxHashMap<WindowId, ()>,
}

impl Default for WinitApp {
    fn default() -> Self {
        Self {
            event_loop: EventLoopBuilder::<WinitBindingAction>::with_user_event().build(),

            window_renderer: FxHashMap::default(),
        }
    }
}

impl WinitApp {
    pub fn run(self) {
        self.event_loop
            .run(move |event, window_target, control_flow| {
                *control_flow = ControlFlow::Wait;

                // let winit_plugin = engine
                //     .get_plugins_mut()
                //     .get_mut::<WinitPlugin>()
                //     .expect("no winit plugin");

                // winit_plugin.process_queue(window_target, control_flow);

                match event {
                    WinitEvent::WindowEvent { event, window_id } => {
                        // winit_plugin.handle_event(window_target, window_id, event, control_flow);
                    }

                    WinitEvent::RedrawRequested(window_id) => {
                        // winit_plugin.render(window_id);
                        // // TODO: limit redraws only to the windows that show visual changes
                        // windows.iter_mut().for_each(|(window_id, window)| {
                        //     window.request_redraw();
                        // });
                    }

                    WinitEvent::UserEvent(action) => match action {
                        WinitBindingAction::CreateWindow(builder_fn, callback) => {
                            let window_builder = (builder_fn)();

                            tracing::trace!("creating window: {:?}", window_builder);

                            callback.call(WinitWindowHandle::new(
                                window_builder
                                    .build(window_target)
                                    .expect("failed to create window"),
                            ));
                        }
                    },

                    _ => (),
                }
            });
    }
}

pub enum WinitBindingAction {
    CreateWindow(
        Box<dyn FnOnce() -> WindowBuilder + Send>,
        Callback<WinitWindowHandle>,
    ),
}
