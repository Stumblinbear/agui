use std::{cell::RefCell, future::Future, rc::Rc};

use agui_core::callback::Callback;
use agui_renderer::RenderWindow;
use futures::{executor::LocalPool, task::LocalSpawnExt};
use rustc_hash::FxHashMap;
use winit::{
    event::Event as WinitEvent,
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{WindowBuilder, WindowId},
};

use crate::{WinitWindowEvent, WinitWindowHandle};

type CreateWinitWindowFn = dyn FnOnce(&winit::window::Window) -> Box<dyn Future<Output = Box<dyn RenderWindow>> + '_>
    + Send;

type WindowRendererSlot = Rc<RefCell<Option<Box<dyn RenderWindow>>>>;

pub enum WinitBindingAction {
    CreateWindow(
        Box<dyn FnOnce() -> WindowBuilder + Send>,
        Box<CreateWinitWindowFn>,
        Callback<WinitWindowHandle>,
    ),
}

pub struct WinitApp {
    pub event_loop: EventLoop<WinitBindingAction>,

    window_events: FxHashMap<WindowId, async_channel::Sender<WinitWindowEvent>>,
    window_renderer: FxHashMap<WindowId, WindowRendererSlot>,
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
        let mut task_pool = LocalPool::new();

        self.event_loop
            .run(move |event, window_target, control_flow| {
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
                        if let Some(renderer) = self.window_renderer.get_mut(&window_id) {
                            if let Ok(mut renderer) = renderer.try_borrow_mut() {
                                if let Some(renderer) = renderer.as_mut() {
                                    renderer.render();
                                } else {
                                    tracing::error!("window renderer was not created");
                                }
                            } else {
                                tracing::warn!("renderer for window is not yet ready");
                            }
                        } else {
                            tracing::warn!("no renderer for window {:?}", window_id);
                        }
                    }

                    WinitEvent::UserEvent(action) => match action {
                        WinitBindingAction::CreateWindow(builder_fn, renderer_fn, callback) => {
                            let window_builder = (builder_fn)();

                            tracing::trace!("creating window: {:?}", window_builder);

                            let window = window_builder
                                .with_visible(false)
                                .build(window_target)
                                .expect("failed to create window");

                            let (events_tx, events_rx) = async_channel::unbounded();

                            self.window_events.insert(window.id(), events_tx);

                            let window_renderer = Rc::new(RefCell::new(None));

                            self.window_renderer
                                .insert(window.id(), Rc::clone(&window_renderer));

                            let window = WinitWindowHandle::new(window, events_rx);

                            callback.call(window.clone());

                            #[allow(clippy::await_holding_refcell_ref)]
                            task_pool
                                .spawner()
                                .spawn_local(async move {
                                    let mut window_renderer = window_renderer.borrow_mut();

                                    let renderer_fut = Box::into_pin((renderer_fn)(&window));

                                    let renderer = renderer_fut.await;

                                    let notifier = renderer.render_notifier();

                                    window_renderer.replace(renderer);

                                    drop(window_renderer);

                                    while let Ok(()) = notifier.recv().await {
                                        window.request_redraw();
                                    }
                                })
                                .expect("failed to spawn render notifier task");
                        }
                    },

                    _ => (),
                }

                if task_pool.try_run_one() {
                    *control_flow = ControlFlow::Poll;
                } else {
                    *control_flow = ControlFlow::Wait;
                }
            });
    }
}
