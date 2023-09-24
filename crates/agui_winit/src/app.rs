use std::{ops::Deref, sync::mpsc, time::Instant};

use agui_core::{
    callback::Callback,
    element::ElementId,
    engine::Engine,
    render::{renderer::Renderer, RenderViewId},
    unit::Offset,
    widget::Widget,
};
use agui_macros::build;
use rustc_hash::FxHashMap;
use winit::{
    dpi::PhysicalPosition,
    event::{Event as WinitEvent, MouseScrollDelta, WindowEvent as WinitWindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder, WindowId},
};

use crate::{
    event::WindowEvent, handle::WinitWindowHandle, windowing_controller::WinitWindowingController,
};

pub struct App<R> {
    event_loop: EventLoop<()>,

    window_tx: mpsc::Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,
    window_rx: mpsc::Receiver<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,

    engine: Engine,

    renderer: R,

    windows: FxHashMap<WindowId, WinitWindowHandle>,
    render_view_window: FxHashMap<RenderViewId, WindowId>,
    window_render_view: FxHashMap<WindowId, RenderViewId>,
}

impl<R> Deref for App<R> {
    type Target = Engine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl<R> App<R>
where
    R: Renderer<Target = Window>,
{
    pub fn with_renderer(renderer: R) -> Self {
        let (window_tx, window_rx) = mpsc::channel();

        Self {
            event_loop: EventLoopBuilder::<()>::with_user_event().build(),

            window_tx,
            window_rx,

            engine: Engine::default(),

            renderer,

            windows: FxHashMap::default(),
            render_view_window: FxHashMap::default(),
            window_render_view: FxHashMap::default(),
        }
    }

    pub fn run(mut self, widget: Widget) {
        self.engine.set_root(build! {
            <WinitWindowingController> {
                tx: self.window_tx.clone(),
                child: self.renderer.build(widget),
            }
        });

        self.event_loop
            .run_return(move |event, window_target, control_flow| {
                while let Ok((element_id, builder, callback)) = self.window_rx.try_recv() {
                    tracing::debug!("creating window for element: {:?}", element_id);

                    let window = WinitWindowHandle::new(builder.build(window_target).unwrap());

                    let size = window.inner_size();

                    let window_id = window.id();

                    let render_view_id = self.engine.get_render_view_manager().get_context(element_id).expect("no render context");

                    self.renderer.create_context(&self.engine, render_view_id, &window, size.width, size.height);

                    self.windows.insert(window_id, window.clone());
                    self.render_view_window.insert(render_view_id, window_id);
                    self.window_render_view.insert(window_id, render_view_id);

                    callback.call(window);
                }

                match event {
                    WinitEvent::WindowEvent { event, window_id } => {
                        match event {
                            WinitWindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                            WinitWindowEvent::Destroyed => {
                                self.windows.remove(&window_id);
                                self.render_view_window.remove(&self.window_render_view.remove(&window_id).unwrap());
                            }

                            WinitWindowEvent::Resized(size) => {
                                if let Some(window) = self.windows.get_mut(&window_id) {
                                    self.renderer.resize(&self.engine, *self.window_render_view.get(&window.id()).unwrap(), size.width, size.height);
                                    window.request_redraw();
                                } else {
                                    tracing::error!(
                                        "a redraw was requested, but the window doesn't exist: {:?}",
                                        window_id
                                    );
                                }
                            }

                            WinitWindowEvent::Moved(pos) => {
                                let window_pos = Offset {
                                    x: pos.x as f32,
                                    y: pos.y as f32,
                                };

                                // self.manager.fire_event(window_pos);
                                // self.manager
                                //     .set_global::<WindowPosition, _>(move |state| *state = window_pos);
                            }

                            WinitWindowEvent::Focused(focused) => {
                                let window_focused = focused;

                                // self.manager.fire_event(window_focused);
                                // self.manager
                                //     .set_global::<WindowFocus, _>(move |state| *state = window_focused);
                            }

                            WinitWindowEvent::CursorMoved { position: pos, .. } => {
                                let mouse_pos = Some(Offset {
                                    x: pos.x as f32,
                                    y: pos.y as f32,
                                });

                                // self.manager.fire_event(mouse_pos);
                                // self.manager
                                //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                                // self.manager
                                //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                            }

                            WinitWindowEvent::CursorLeft { .. } => {
                                // let mouse_pos = None;

                                // self.manager.fire_event(mouse_pos);
                                // self.manager
                                //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                                // self.manager
                                //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                            }

                            WinitWindowEvent::MouseWheel { delta, .. } => {
                                let scroll = match delta {
                                    MouseScrollDelta::LineDelta(x, y) => Offset { x, y },

                                    MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => {
                                        Offset {
                                            x: x as f32,
                                            y: y as f32,
                                        }
                                    }
                                };

                                // self.manager.fire_event(scroll);
                                // self.manager
                                //     .set_global::<Scroll, _>(move |state| *state = scroll);
                            }

                            WinitWindowEvent::MouseInput {
                                button,
                                state: value,
                                ..
                            } => {
                                // let button_state = match value {
                                //     ElementState::Pressed => MouseButtonState::Pressed,
                                //     ElementState::Released => MouseButtonState::Released,
                                // };

                                // self.manager.fire_event(match button {
                                //     winit::event::MouseButton::Left => MouseButton::Left(button_state),
                                //     winit::event::MouseButton::Right => MouseButton::Right(button_state),
                                //     winit::event::MouseButton::Middle => MouseButton::Middle(button_state),
                                //     winit::event::MouseButton::Other(i) => {
                                //         MouseButton::Other(i, button_state)
                                //     }
                                // });

                                // self.manager
                                //     .set_global::<MouseButtons, _>(move |state| match button {
                                //         winit::event::MouseButton::Left => state.left = button_state,
                                //         winit::event::MouseButton::Right => state.right = button_state,
                                //         winit::event::MouseButton::Middle => state.middle = button_state,
                                //         winit::event::MouseButton::Other(i) => {
                                //             state.other.insert(i, button_state);
                                //         }
                                //     });

                                // self.manager
                                //     .set_global::<Mouse, _>(move |state| match button {
                                //         winit::event::MouseButton::Left => state.button.left = button_state,
                                //         winit::event::MouseButton::Right => {
                                //             state.button.right = button_state
                                //         }
                                //         winit::event::MouseButton::Middle => {
                                //             state.button.middle = button_state
                                //         }
                                //         winit::event::MouseButton::Other(i) => {
                                //             state.button.other.insert(i, button_state);
                                //         }
                                //     });
                            }

                            WinitWindowEvent::ReceivedCharacter(c) => {
                                // self.manager.fire_event(KeyboardCharacter(c));
                            }

                            WinitWindowEvent::KeyboardInput { input, .. } => {
                                if let Some(key) = input.virtual_keycode {
                                    // let key: KeyCode = unsafe { std::mem::transmute(key as u32) };

                                    // let key_state = match input.state {
                                    //     ElementState::Pressed => KeyState::Pressed,
                                    //     ElementState::Released => KeyState::Released,
                                    // };

                                    // self.manager.fire_event(KeyboardInput(key, key_state));

                                    // self.manager.set_global::<Keyboard, _>(move |state| {
                                    //     state.keys.insert(key, key_state);
                                    // });
                                }
                            }

                            WinitWindowEvent::ModifiersChanged(modifiers) => {
                                // self.manager.set_global::<Keyboard, _>(move |state| {
                                //     state.modifiers = unsafe { mem::transmute(modifiers) };
                                // });
                            }

                            _ => {}
                        }

                        if let Some(handle) = self.windows.get_mut(&window_id) {
                            handle.notify(&WindowEvent::from(event));
                        }
                    }

                    WinitEvent::MainEventsCleared => {
                        self.windows.iter().for_each(|(_, window)| {
                            window.request_redraw();
                        });
                    }

                    WinitEvent::RedrawRequested(window_id) => {
                        if let Some(render_view_id) = self.window_render_view.get(&window_id) {
                            self.renderer.render(*render_view_id);
                        } else {
                            tracing::error!(
                                "a redraw was requested, but the window doesn't exist: {:?}",
                                window_id
                            );
                        }
                    }

                    _ => (),
                }

                let now = Instant::now();

                let events = self.engine.update();

                if !events.is_empty() {
                    tracing::info!("updated in: {:?}", Instant::now().duration_since(now));

                    // TODO: limit redraws only to the windows that show visual changes
                    // Maybe check if any redrawn widgets have a Window InheritedWidget?
                    self.windows.iter_mut().for_each(|(window_id, window)| {
                        self.renderer.redraw(&self.engine, *self.window_render_view.get(window_id).unwrap(), &events);

                        window.request_redraw();
                    });
                }
            });
    }
}
