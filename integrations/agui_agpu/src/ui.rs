use std::{
    io, mem,
    ops::{Deref, DerefMut},
};

use agpu::{
    winit::winit::{
        self,
        dpi::PhysicalPosition,
        event::{ElementState, Event as WinitEvent, MouseScrollDelta, WindowEvent},
    },
    Event, Gpu, GpuProgram,
};
use agui::{
    engine::Engine,
    unit::{Font, Point, Size},
    widgets::{
        plugins::{event::EventPluginEngineExt, global::GlobalPluginExt},
        state::{
            keyboard::{KeyCode, KeyState, Keyboard, KeyboardCharacter, KeyboardInput},
            mouse::{Mouse, MouseButton, MouseButtonState, MouseButtons, MousePos, Scroll},
            window::{WindowFocus, WindowPosition, WindowSize},
        },
    },
};
use glyph_brush_draw_cache::ab_glyph::InvalidFont;

use crate::render::RenderEngine;

pub struct UI {
    engine: Engine,
    renderer: RenderEngine,
}

impl Deref for UI {
    type Target = Engine;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl DerefMut for UI {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.engine
    }
}

impl UI {
    pub fn from_program(program: &GpuProgram) -> Self {
        let surface = program.viewport.sc_desc.borrow();

        Self::using_gpu(
            &program.gpu,
            Size {
                width: surface.width as f32,
                height: surface.height as f32,
            },
        )
    }

    pub fn using_gpu(gpu: &Gpu, size: Size) -> Self {
        Self {
            engine: Engine::new(),
            renderer: RenderEngine::new(gpu, size),
        }
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> Result<Font, InvalidFont> {
        self.engine.load_font_bytes(bytes)
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<Font> {
        self.engine.load_font_file(filename)
    }

    pub fn redraw(&mut self) {
        self.renderer.redraw(&self.engine);

        // print_tree(&self.engine);
    }

    pub fn handle_event(&mut self, event: Event<'_, ()>, program: &GpuProgram) {
        if let Some(_widget_events) = self.engine.update() {
            self.redraw();

            // If the program is not already demanding a specific framerate, request a redraw
            if program.time.is_none() {
                program.viewport.request_redraw();
            }
        }

        if let Event::RedrawFrame(frame) = event {
            self.renderer.render(frame);
        } else if let Event::Winit(WinitEvent::WindowEvent { event, .. }) = event {
            match event {
                WindowEvent::Resized(size) => {
                    self.renderer.set_size(Size {
                        width: size.width as f32,
                        height: size.height as f32,
                    });

                    let window_size = WindowSize {
                        width: size.width as f32,
                        height: size.height as f32,
                    };

                    self.fire_event(window_size);
                    self.set_global::<WindowSize, _>(move |state| *state = window_size);
                }

                WindowEvent::Moved(pos) => {
                    let window_pos = WindowPosition {
                        x: pos.x as f32,
                        y: pos.y as f32,
                    };

                    self.fire_event(window_pos);
                    self.set_global::<WindowPosition, _>(move |state| *state = window_pos);
                }

                WindowEvent::Focused(focused) => {
                    let window_focused = WindowFocus(focused);

                    self.fire_event(window_focused);
                    self.set_global::<WindowFocus, _>(move |state| *state = window_focused);
                }

                WindowEvent::CursorMoved { position: pos, .. } => {
                    let mouse_pos = MousePos(Some(Point {
                        x: pos.x as f32,
                        y: pos.y as f32,
                    }));

                    self.fire_event(mouse_pos);
                    self.set_global::<MousePos, _>(move |state| *state = mouse_pos);
                    self.set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                }

                WindowEvent::CursorLeft { .. } => {
                    let mouse_pos = MousePos(None);

                    self.fire_event(mouse_pos);
                    self.set_global::<MousePos, _>(move |state| *state = mouse_pos);
                    self.set_global::<Mouse, _>(move |state| state.pos = mouse_pos);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll = Scroll(match delta {
                        MouseScrollDelta::LineDelta(x, y) => Point { x, y },

                        MouseScrollDelta::PixelDelta(PhysicalPosition { x, y }) => Point {
                            x: x as f32,
                            y: y as f32,
                        },
                    });

                    self.fire_event(scroll);
                    self.set_global::<Scroll, _>(move |state| *state = scroll);
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

                    self.fire_event(match button {
                        winit::event::MouseButton::Left => MouseButton::Left(button_state),
                        winit::event::MouseButton::Right => MouseButton::Right(button_state),
                        winit::event::MouseButton::Middle => MouseButton::Middle(button_state),
                        winit::event::MouseButton::Other(i) => MouseButton::Other(i, button_state),
                    });

                    self.set_global::<MouseButtons, _>(move |state| match button {
                        winit::event::MouseButton::Left => state.left = button_state,
                        winit::event::MouseButton::Right => state.right = button_state,
                        winit::event::MouseButton::Middle => state.middle = button_state,
                        winit::event::MouseButton::Other(i) => {
                            state.other.insert(i, button_state);
                        }
                    });

                    self.set_global::<Mouse, _>(move |state| match button {
                        winit::event::MouseButton::Left => state.button.left = button_state,
                        winit::event::MouseButton::Right => state.button.right = button_state,
                        winit::event::MouseButton::Middle => state.button.middle = button_state,
                        winit::event::MouseButton::Other(i) => {
                            state.button.other.insert(i, button_state);
                        }
                    });
                }

                WindowEvent::ReceivedCharacter(c) => {
                    self.fire_event(KeyboardCharacter(c));
                }

                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(key) = input.virtual_keycode {
                        let key: KeyCode = unsafe { mem::transmute(key as u32) };

                        let key_state = match input.state {
                            ElementState::Pressed => KeyState::Pressed,
                            ElementState::Released => KeyState::Released,
                        };

                        self.fire_event(KeyboardInput(key, key_state));

                        self.set_global::<Keyboard, _>(move |state| {
                            state.keys.insert(key, key_state);
                        });
                    }
                }

                WindowEvent::ModifiersChanged(modifiers) => {
                    self.set_global::<Keyboard, _>(move |state| {
                        state.modifiers = unsafe { mem::transmute(modifiers) };
                    });
                }

                _ => {}
            }
        }
    }
}
