use agui_core::callback::Callback;
use agui_renderer::RenderWindow;
use rustc_hash::FxHashMap;
use winit::{
    event::Event as WinitEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{WindowBuilder, WindowId},
};

use crate::{WinitWindowEvent, WinitWindowHandle};

type BoxedRenderer = Box<dyn RenderWindow<Target = WinitWindowHandle>>;

pub struct WinitApp {
    pub event_loop: EventLoop<WinitBindingAction>,

    window_events: FxHashMap<WindowId, async_channel::Sender<WinitWindowEvent>>,
    window_renderer: FxHashMap<WindowId, BoxedRenderer>,
}

impl Default for WinitApp {
    fn default() -> Self {
        Self {
            event_loop: EventLoopBuilder::<WinitBindingAction>::with_user_event().build(),

            window_events: FxHashMap::default(),
            window_renderer: FxHashMap::default(),
        }
    }
}

impl WinitApp {
    pub fn run(mut self) {
        self.event_loop
            .run(move |event, window_target, control_flow| {
                *control_flow = ControlFlow::Wait;

                match event {
                    WinitEvent::WindowEvent { event, window_id } => {
                        if let Some(event) = event.to_static() {
                            if let Some(events_tx) = self.window_events.get(&window_id) {
                                if events_tx
                                    .send_blocking(WinitWindowEvent::from(event))
                                    .is_err()
                                {
                                    tracing::error!("window event channel is closed");
                                }

                                if events_tx.is_closed() {
                                    tracing::debug!("closing window");

                                    self.window_events.remove(&window_id);
                                    self.window_renderer.remove(&window_id);
                                }
                            } else {
                                tracing::warn!("no renderer for window {:?}", window_id);
                            }
                        }
                    }

                    WinitEvent::RedrawRequested(window_id) => {
                        if let Some(renderer) = self.window_renderer.get(&window_id) {
                            renderer.render();
                        } else {
                            tracing::warn!("no renderer for window {:?}", window_id);
                        }
                    }

                    WinitEvent::UserEvent(action) => match action {
                        WinitBindingAction::CreateWindow(builder_fn, renderer_fn, callback) => {
                            let window_builder = (builder_fn)();

                            tracing::trace!("creating window: {:?}", window_builder);

                            let (events_tx, events_rx) = async_channel::unbounded();

                            let window_handle = WinitWindowHandle::new(
                                window_builder
                                    .build(window_target)
                                    .expect("failed to create window"),
                                events_rx,
                            );

                            self.window_events.insert(window_handle.id(), events_tx);

                            self.window_renderer
                                .insert(window_handle.id(), (renderer_fn)(window_handle.clone()));

                            callback.call(window_handle);
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
        Box<dyn FnOnce(WinitWindowHandle) -> BoxedRenderer + Send>,
        Callback<WinitWindowHandle>,
    ),
}
