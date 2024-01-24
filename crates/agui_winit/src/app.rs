use std::{error::Error, future::Future};

use agui_core::callback::Callback;
use agui_renderer::{FrameNotifier, Renderer};
use agui_sync::broadcast;
use rustc_hash::FxHashMap;
use winit::{
    error::OsError,
    event::Event as WinitEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{WindowBuilder, WindowId},
};

use crate::{WinitWindowEvent, WinitWindowHandle};

type CreateWinitWindowFn = dyn FnOnce(
        &winit::window::Window,
        FrameNotifier,
    )
        -> Box<dyn Future<Output = Result<Box<dyn Renderer>, Box<dyn Error + Send + Sync>>> + '_>
    + Send;

#[derive(Debug, thiserror::Error)]
pub enum WinitCreateWindowError {
    #[error("failed to create the window: {0}")]
    Os(#[from] OsError),

    #[error("failed to create the renderer: {0}")]
    Renderer(Box<dyn Error + Send>),
}

pub enum WinitBindingAction {
    CreateWindow(
        Box<dyn FnOnce() -> WindowBuilder + Send>,
        Box<CreateWinitWindowFn>,
        Callback<Result<WinitWindowHandle, WinitCreateWindowError>>,
    ),

    Render(WindowId),
}

pub struct WinitApp {
    pub event_loop: EventLoop<WinitBindingAction>,

    window_events: FxHashMap<WindowId, broadcast::UnboundedSender<WinitWindowEvent>>,
    window_renderer: FxHashMap<WindowId, Box<dyn Renderer>>,
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
        let event_loop_proxy = self.event_loop.create_proxy();

        self.event_loop
            .run(move |event, window_target, control_flow| {
                *control_flow = ControlFlow::Wait;

                match event {
                    WinitEvent::WindowEvent { event, window_id } => {
                        if let Some(event) = event.to_static() {
                            let is_destroyed =
                                matches!(event, winit::event::WindowEvent::Destroyed);

                            if let Some(events_tx) = self.window_events.get_mut(&window_id) {
                                if let Err(err) = futures::executor::block_on(
                                    events_tx.send(WinitWindowEvent::from(event)),
                                ) {
                                    tracing::error!("failed to broadcast winit event: {}", err);
                                }
                            } else {
                                tracing::warn!(?window_id, "no renderer for window");
                            }

                            if is_destroyed {
                                tracing::debug!(?window_id, "window was destroyed");

                                self.window_renderer.remove(&window_id);

                                if let Some(events_tx) = self.window_events.remove(&window_id) {
                                    if !events_tx.close() {
                                        tracing::error!(
                                            ?window_id,
                                            "window event channel was already closed"
                                        )
                                    }
                                }
                            }
                        }
                    }

                    WinitEvent::RedrawRequested(window_id)
                    | WinitEvent::UserEvent(WinitBindingAction::Render(window_id)) => {
                        if let Some(renderer) = self.window_renderer.get_mut(&window_id) {
                            renderer.render();
                        } else {
                            tracing::warn!("no renderer for window {:?}", window_id);
                        }
                    }

                    WinitEvent::UserEvent(WinitBindingAction::CreateWindow(
                        builder_fn,
                        renderer_fn,
                        callback,
                    )) => {
                        let window_builder = (builder_fn)();

                        tracing::trace!("creating window: {:?}", window_builder);

                        let window = match window_builder.with_visible(false).build(window_target) {
                            Ok(window) => window,
                            Err(err) => return callback.call(Err(err.into())),
                        };

                        let (events_tx, _) = broadcast::unbounded();

                        self.window_events.insert(window.id(), events_tx.clone());

                        let window = WinitWindowHandle::new(window, events_tx.clone());

                        let renderer_fut = Box::into_pin((renderer_fn)(
                            &window,
                            FrameNotifier::new({
                                let event_loop_proxy = event_loop_proxy.clone();
                                let window_id = window.id();

                                move || {
                                    let _ = event_loop_proxy
                                        .send_event(WinitBindingAction::Render(window_id));
                                }
                            }),
                        ));

                        // TODO: figure out how to make this actually async
                        let renderer = match futures::executor::block_on(renderer_fut) {
                            Ok(renderer) => renderer,
                            Err(err) => {
                                return callback.call(Err(WinitCreateWindowError::Renderer(err)));
                            }
                        };

                        self.window_renderer.insert(window.id(), renderer);

                        callback.call(Ok(window));
                    }

                    _ => (),
                }
            });
    }
}
