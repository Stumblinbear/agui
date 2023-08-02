use std::{
    ops::{Deref, DerefMut},
    sync::mpsc,
    time::Instant,
};

use agui::{
    element::ElementId,
    manager::WidgetManager,
    prelude::{layout_controller::TextLayoutController, *},
    widgets::state::{
        keyboard::{KeyCode, KeyState},
        mouse::MouseButtonState,
        window::WindowPosition,
    },
};
use fnv::FnvHashMap;
use futures::executor::block_on;
use vello::fello::raw::FontRef;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event as WinitEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    platform::run_return::EventLoopExtRunReturn,
    window::{Window, WindowBuilder, WindowId},
};

use crate::{
    bindings::{VelloTextLayoutDelegate, WinitWindowHandle, WinitWindowingController},
    manager::RenderManager,
};

pub struct AguiProgram {
    event_loop: EventLoop<()>,

    window_tx: mpsc::Sender<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,
    window_rx: mpsc::Receiver<(ElementId, WindowBuilder, Callback<WinitWindowHandle>)>,

    manager: WidgetManager,

    window_contexts: FnvHashMap<WindowId, WindowContext>,
}

struct WindowContext {
    window: Window,

    renderer: RenderManager,
}

impl Deref for AguiProgram {
    type Target = WidgetManager;

    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}

impl DerefMut for AguiProgram {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.manager
    }
}

impl Default for AguiProgram {
    fn default() -> Self {
        let (window_tx, window_rx) = mpsc::channel();

        Self {
            event_loop: EventLoopBuilder::<()>::with_user_event().build(),

            window_tx,
            window_rx,

            manager: WidgetManager::new(),

            window_contexts: FnvHashMap::default(),
        }
    }
}

impl AguiProgram {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_root<W>(&mut self, widget: W)
    where
        W: IntoChild,
    {
        self.manager.set_root(
            WinitWindowingController::new(self.window_tx.clone()).with_child(
                TextLayoutController::new()
                    .with_delegate(VelloTextLayoutDelegate {
                        default_font: FontRef::new(include_bytes!(
                            "../examples/fonts/DejaVuSans.ttf"
                        ))
                        .unwrap(),
                    })
                    .with_child(widget),
            ),
        );
    }

    pub fn run(mut self) {
        self.event_loop
            .run_return(move |event, window_target, control_flow| {
                while let Ok((element_id, builder, callback)) = self.window_rx.try_recv() {
                    tracing::debug!("creating window for element: {:?}", element_id);

                    let window = builder.build(window_target).unwrap();

                    let size = window.inner_size();

                    let window_id = window.id();

                    // TODO: figure out how to only render the window's subtree
                    let mut renderer = block_on(
                        RenderManager::new(self.manager.get_render_context_manager().get_context(element_id).expect("no render context"), &window, size.width, size.height)
                    );

                    renderer.init(&self.manager);

                    let handle = WinitWindowHandle {
                        window_id,

                        title: window.title(),
                    };

                    self.window_contexts
                        .insert(window_id, WindowContext { window, renderer });

                    callback.call(handle);
                }

                match event {
                    WinitEvent::WindowEvent { event, window_id } => {
                        match event {
                            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                            WindowEvent::Destroyed => {
                                self.window_contexts.remove(&window_id);
                            }

                            WindowEvent::Resized(size) => {
                                if let Some(ctx) = self.window_contexts.get_mut(&window_id) {
                                    ctx.renderer.resize(size.width, size.height);
                                    ctx.window.request_redraw();
                                } else {
                                    tracing::error!(
                                        "a redraw was requested, but the window doesn't exist: {:?}",
                                        window_id
                                    );
                                }
                            }

                            WindowEvent::Moved(pos) => {
                                let window_pos = WindowPosition {
                                    x: pos.x as f32,
                                    y: pos.y as f32,
                                };

                                // self.manager.fire_event(window_pos);
                                // self.manager
                                //     .set_global::<WindowPosition, _>(move |state| *state = window_pos);
                            }

                            WindowEvent::Focused(focused) => {
                                let window_focused = focused;

                                // self.manager.fire_event(window_focused);
                                // self.manager
                                //     .set_global::<WindowFocus, _>(move |state| *state = window_focused);
                            }

                            WindowEvent::CursorMoved { position: pos, .. } => {
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

                            WindowEvent::CursorLeft { .. } => {
                                // let mouse_pos = None;

                                // self.manager.fire_event(mouse_pos);
                                // self.manager
                                //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                                // self.manager
                                //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                            }

                            WindowEvent::MouseWheel { delta, .. } => {
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

                            WindowEvent::MouseInput {
                                button,
                                state: value,
                                ..
                            } => {
                                let button_state = match value {
                                    ElementState::Pressed => MouseButtonState::Pressed,
                                    ElementState::Released => MouseButtonState::Released,
                                };

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

                            WindowEvent::ReceivedCharacter(c) => {
                                // self.manager.fire_event(KeyboardCharacter(c));
                            }

                            WindowEvent::KeyboardInput { input, .. } => {
                                if let Some(key) = input.virtual_keycode {
                                    let key: KeyCode = unsafe { std::mem::transmute(key as u32) };

                                    let key_state = match input.state {
                                        ElementState::Pressed => KeyState::Pressed,
                                        ElementState::Released => KeyState::Released,
                                    };

                                    // self.manager.fire_event(KeyboardInput(key, key_state));

                                    // self.manager.set_global::<Keyboard, _>(move |state| {
                                    //     state.keys.insert(key, key_state);
                                    // });
                                }
                            }

                            WindowEvent::ModifiersChanged(modifiers) => {
                                // self.manager.set_global::<Keyboard, _>(move |state| {
                                //     state.modifiers = unsafe { mem::transmute(modifiers) };
                                // });
                            }

                            _ => {}
                        }
                    }

                    WinitEvent::MainEventsCleared => {
                        self.window_contexts.iter().for_each(|(_, ctx)| {
                            ctx.window.request_redraw();
                        });
                    }

                    WinitEvent::RedrawRequested(window_id) => {
                        if let Some(ctx) = self.window_contexts.get_mut(&window_id) {
                            ctx.renderer.render();
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

                let events = self.manager.update();

                if !events.is_empty() {
                    tracing::info!("updated in: {:?}", Instant::now().duration_since(now));

                    // TODO: limit redraws only to the windows that show visual changes
                    // Maybe check if any redrawn widgets have a Window InheritedWidget?
                    self.window_contexts.iter_mut().for_each(|(_, ctx)| {
                        ctx.renderer.redraw(&self.manager, &events);

                        ctx.window.request_redraw();
                    });
                }
            });
    }
}
