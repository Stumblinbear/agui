use std::{
    mem,
    ops::{Deref, DerefMut},
    time::Instant,
};

use agui::{
    manager::WidgetManager,
    unit::{Offset, Size},
    widget::IntoWidget,
    widgets::{
        primitives::layout_controller::TextLayoutController,
        state::{
            keyboard::{KeyCode, KeyState},
            mouse::{MouseButtonState, MousePos, Scroll},
            window::{WindowFocus, WindowPosition},
        },
    },
};
use futures::executor::block_on;
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, Event as WinitEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

use crate::{manager::RenderManager, text_layout::VelloTextLayoutDelegate};

pub struct AguiProgram {
    event_loop: EventLoop<()>,
    window: Window,

    manager: WidgetManager,
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

impl AguiProgram {
    pub fn new(title: &str, size: Size) -> Self {
        let event_loop = EventLoopBuilder::<()>::with_user_event().build();

        let window = WindowBuilder::new()
            .with_title(title)
            .with_decorations(true)
            .with_resizable(true)
            .with_transparent(false)
            .with_inner_size(winit::dpi::PhysicalSize {
                width: size.width,
                height: size.height,
            })
            .build(&event_loop)
            .unwrap();

        let size = window.inner_size();

        let renderer = block_on(RenderManager::new(&window, size.width, size.height));

        Self {
            event_loop,
            window,

            manager: WidgetManager::new(),
            renderer,
        }
    }

    pub fn set_root<W>(&mut self, widget: W)
    where
        W: IntoWidget,
    {
        self.manager.set_root(
            TextLayoutController::new()
                .with_delegate(VelloTextLayoutDelegate)
                .with_child(widget),
        );
    }

    pub fn run(mut self) -> ! {
        self.event_loop.run(move |event, _, control_flow| {
            match event {
                WinitEvent::WindowEvent {
                    event, window_id, ..
                } => {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                        WindowEvent::Resized(size) => {
                            self.renderer.resize(size.width, size.height);

                            self.window.request_redraw();
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
                            let window_focused = WindowFocus(focused);

                            // self.manager.fire_event(window_focused);
                            // self.manager
                            //     .set_global::<WindowFocus, _>(move |state| *state = window_focused);
                        }

                        WindowEvent::CursorMoved { position: pos, .. } => {
                            let mouse_pos = MousePos(Some(Offset {
                                x: pos.x as f32,
                                y: pos.y as f32,
                            }));

                            // self.manager.fire_event(mouse_pos);
                            // self.manager
                            //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                            // self.manager
                            //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                        }

                        WindowEvent::CursorLeft { .. } => {
                            let mouse_pos = MousePos(None);

                            // self.manager.fire_event(mouse_pos);
                            // self.manager
                            //     .set_global::<MousePos, _>(move |state| *state = mouse_pos);
                            // self.manager
                            //     .set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                        }

                        WindowEvent::MouseWheel { delta, .. } => {
                            let scroll = Scroll(match delta {
                                MouseScrollDelta::LineDelta(x, y) => Offset { x, y },

                                MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => Offset {
                                    x: x as f32,
                                    y: y as f32,
                                },
                            });

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
                                let key: KeyCode = unsafe { mem::transmute(key as u32) };

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
                    self.window.request_redraw();
                }

                WinitEvent::RedrawRequested(..) => {
                    self.renderer.render();
                }

                _ => (),
            }

            let now = Instant::now();

            let events = self.manager.update();

            if !events.is_empty() {
                tracing::info!("updated in: {:?}", Instant::now().duration_since(now));

                self.renderer.redraw(&self.manager, &events);

                self.window.request_redraw();
            }
        });
    }
}
