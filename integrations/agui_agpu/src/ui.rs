use std::{
    io, mem,
    ops::{Deref, DerefMut},
};

use agpu::{
    winit::winit::event::{
        ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    Event, Frame, GpuHandle, GpuProgram,
};
use agui::{
    engine::Engine,
    font::Font,
    unit::{Point, Size},
    widgets::state::{
        keyboard::{KeyCode, KeyState, Keyboard, KeyboardInput},
        mouse::{Mouse, MouseButtonState, Scroll},
        window::{WindowFocus, WindowPosition, WindowSize},
    },
};
use glyph_brush_draw_cache::ab_glyph::InvalidFont;

use crate::render::RenderEngine;

pub struct UI<'ui> {
    engine: Engine<'ui>,
    renderer: RenderEngine,
}

impl<'ui> Deref for UI<'ui> {
    type Target = Engine<'ui>;

    fn deref(&self) -> &Self::Target {
        &self.engine
    }
}

impl<'ui> DerefMut for UI<'ui> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.engine
    }
}

impl<'ui> UI<'ui> {
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

    pub fn using_gpu(gpu: &GpuHandle, size: Size) -> Self {
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

    pub fn redraw(&'ui mut self) {
        self.renderer.redraw(&self.engine);
    }

    pub fn render(&mut self, frame: Frame) {
        self.renderer.render(frame);
    }

    pub fn handle_event(&'ui mut self, event: Event<'_, ()>, program: &GpuProgram) {
        if let Some(_widget_events) = self.engine.update() {
            self.redraw();

            // If the program is not already demanding a specific framerate, request a redraw
            if program.time.is_none() {
                program.viewport.request_redraw();
            }

            return;
        }

        if let Event::RedrawFrame(frame) = event {
            self.render(frame);
        } else if let Event::Winit(WinitEvent::WindowEvent { event, .. }) = event {
            match event {
                WindowEvent::Resized(size) => {
                    self.renderer.set_size(Size {
                        width: size.width as f32,
                        height: size.height as f32,
                    });

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
