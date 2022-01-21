use std::mem;

use agpu::{
    winit::winit::event::{
        ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    Event, GpuProgram,
};
use agui::{
    engine::Engine,
    unit::Point,
    widgets::{
        state::{
            keyboard::{KeyCode, KeyState, Keyboard, KeyboardInput},
            mouse::{Mouse, MouseButtonState, Scroll},
            window::{WindowFocus, WindowPosition, WindowSize},
        },
        AppSettings,
    },
};

use crate::{AgpuPicture, AgpuRenderer};

pub trait AgpuEngineExt {
    fn handle_event(&mut self, event: Event<'_, ()>, program: &GpuProgram);
}

impl AgpuEngineExt for Engine<'_, AgpuRenderer, AgpuPicture> {
    fn handle_event(&mut self, event: Event<'_, ()>, program: &GpuProgram) {
        if self.update() {
            // If the program is not already demanding a specific framerate, request a redraw
            if program.time.is_none() {
                program.viewport.request_redraw();
            }
        }

        if let Event::RedrawFrame(frame) = event {
            self.redraw();

            self.get_renderer().render(frame);
        } else if let Event::Winit(WinitEvent::WindowEvent { event, .. }) = event {
            match event {
                WindowEvent::Resized(size) => {
                    if let Some(state) = self.try_use_global::<WindowSize>() {
                        let mut state = state.write();

                        state.width = size.width as f32;
                        state.height = size.height as f32;
                    }
                }

                WindowEvent::Moved(pos) => {
                    if let Some(state) = self.try_use_global::<WindowPosition>() {
                        let mut state = state.write();

                        state.x = pos.x as f32;
                        state.y = pos.y as f32;
                    }
                }

                WindowEvent::ReceivedCharacter(c) => {
                    if let Some(state) = self.try_use_global::<KeyboardInput>() {
                        **state.write() = c;
                    }
                }

                WindowEvent::Focused(focused) => {
                    if let Some(state) = self.try_use_global::<WindowFocus>() {
                        **state.write() = focused;
                    }
                }

                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(state) = self.try_use_global::<Keyboard>() {
                        let mut state = state.write();

                        if let Some(key) = input.virtual_keycode {
                            let key: KeyCode = unsafe { mem::transmute(key as u32) };

                            match input.state {
                                ElementState::Pressed => {
                                    state.keys.insert(key, KeyState::Pressed);
                                }
                                ElementState::Released => {
                                    state.keys.insert(key, KeyState::Released);
                                }
                            }
                        }
                    }
                }

                WindowEvent::ModifiersChanged(modifiers) => {
                    if let Some(state) = self.try_use_global::<Keyboard>() {
                        let mut state = state.write();

                        state.modifiers = unsafe { mem::transmute(modifiers) };
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(state) = self.try_use_global::<Mouse>() {
                        let mut state = state.write();

                        match state.pos {
                            Some(ref mut pos) => {
                                pos.x = position.x as f32;
                                pos.y = position.y as f32;
                            }
                            None => {
                                state.pos = Some(Point {
                                    x: position.x as f32,
                                    y: position.y as f32,
                                });
                            }
                        }
                    }
                }

                WindowEvent::CursorLeft { .. } => {
                    if let Some(state) = self.try_use_global::<Mouse>() {
                        let mut state = state.write();

                        state.pos = None;
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some(state) = self.try_use_global::<Scroll>() {
                        let mut state = state.write();

                        match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                state.delta.x = x;
                                state.delta.y = y;
                            }
                            MouseScrollDelta::PixelDelta(position) => {
                                state.delta.x = position.x as f32;
                                state.delta.y = position.y as f32;
                            }
                        }
                    }
                }

                WindowEvent::MouseInput {
                    button,
                    state: value,
                    ..
                } => {
                    if let Some(state) = self.try_use_global::<Mouse>() {
                        let mut state = state.write();

                        let button = match button {
                            MouseButton::Left => &mut state.button.left,
                            MouseButton::Middle => &mut state.button.middle,
                            MouseButton::Right => &mut state.button.right,
                            MouseButton::Other(_) => {
                                return;
                            }
                        };

                        match value {
                            ElementState::Pressed => {
                                if *button == MouseButtonState::Released {
                                    *button = MouseButtonState::Pressed;
                                } else {
                                    *button = MouseButtonState::Held;
                                }
                            }
                            ElementState::Released => {
                                *button = MouseButtonState::Released;
                            }
                        };
                    }
                }

                _ => {}
            }
        }
    }
}
