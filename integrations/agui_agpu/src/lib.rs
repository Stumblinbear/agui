use std::{io, mem};

use agpu::{
    winit::winit::event::{
        ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    Event, Frame, GpuProgram, RenderPipeline,
};

use agui::{
    context::{Notify, WidgetContext},
    engine::{plugin::EnginePlugin, Engine},
    render::RenderManager,
    widget::{WidgetId, WidgetRef},
    widgets::{
        plugins::{hovering::HoveringPlugin, timeout::TimeoutPlugin},
        primitives::{FontDescriptor, Fonts},
        state::{
            keyboard::{KeyCode, KeyState, Keyboard, KeyboardInput},
            mouse::{Mouse, MouseButtonState, Scroll, XY},
            window::{WindowFocus, WindowPosition, WindowSize},
        },
        AppSettings,
    },
};
use render::RenderContext;

mod render;

pub struct UI<'ui> {
    engine: Engine<'ui>,
    render: RenderManager,

    pipeline: RenderPipeline,

    ctx: RenderContext,
}

impl<'ui> UI<'ui> {
    pub fn new(program: &GpuProgram) -> Self {
        let mut engine = Engine::default();

        let app_settings = engine.get_context_mut().init_global(|| AppSettings {
            width: program.viewport.inner_size().width as f32,
            height: program.viewport.inner_size().height as f32,
        });

        let ctx = RenderContext::new(program, Notify::clone(&app_settings));

        program.on_resize(move |_, w, h| {
            if w == 0 && h == 0 {
                return;
            }

            let mut app_settings = app_settings.write();

            app_settings.width = w as f32;
            app_settings.height = h as f32;
        });

        Self {
            engine,
            render: RenderManager::default(),

            pipeline: program.gpu.new_pipeline("agui clear pipeline").create(),

            ctx,
        }
    }

    pub fn with_default(program: &GpuProgram) -> Self {
        let mut ui = Self::new(program);

        ui.init_plugin(HoveringPlugin);
        ui.init_plugin(TimeoutPlugin);

        ui
    }

    /// Initializes a UI plugin.
    ///
    /// # Panics
    ///
    /// Will panic if you attempt to initialize a plugin a second time.
    pub fn init_plugin<P>(&mut self, plugin: P)
    where
        P: EnginePlugin,
    {
        self.engine.init_plugin(plugin)
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> FontDescriptor {
        let (font, font_arc) = self
            .get_context_mut()
            .use_global::<Fonts, _>(Fonts::default)
            .write()
            .load_bytes(bytes);

        // self.get_pass_mut::<TextRenderPass>().add_font(font_arc);

        font
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<FontDescriptor> {
        match self
            .get_context_mut()
            .use_global::<Fonts, _>(Fonts::default)
            .write()
            .load_file(filename)
        {
            Ok((font, font_arc)) => {
                // self.get_pass_mut::<TextRenderPass>().add_font(font_arc);

                Ok(font)
            }
            Err(err) => Err(err),
        }
    }

    pub fn get_context(&self) -> &WidgetContext<'ui> {
        self.engine.get_context()
    }
    pub fn get_context_mut(&mut self) -> &mut WidgetContext<'ui> {
        self.engine.get_context_mut()
    }

    pub fn set_root(&mut self, widget: WidgetRef) {
        self.engine.add(None, widget);
    }

    pub fn add(&mut self, parent_id: Option<WidgetId>, widget: WidgetRef) {
        self.engine.add(parent_id, widget);
    }

    pub fn remove(&mut self, widget_id: WidgetId) {
        self.engine.remove(widget_id);
    }

    pub fn update(&mut self) -> bool {
        let mut events = Vec::new();

        self.engine.update(&mut events);

        if events.is_empty() {
            return false;
        }

        self.ctx.update();

        self.render.update(&self.engine, &events);

        self.render
            .take_dirty()
            .iter()
            .map(|render_id| (*render_id, self.render.get_tree().get(*render_id)))
            .for_each(|(render_id, node)| {
                if let Some(node) = node {
                    println!("{:?} {:?} {:?}", node.widget_id, render_id, node.rect);
                } else {
                    println!("destroyed");
                }
            });

        true
    }

    pub fn render(&self, mut frame: Frame) {
        // We complete rendering by first clearing the screen, then creating the layer buffer based on
        // clipping masks, before finally rendering the actual widgets through the added render passes.
        frame
            .render_pass_cleared("agui clear pass", 0x101010FF)
            .with_pipeline(&self.pipeline)
            .begin();
    }

    pub fn handle(&mut self, event: Event<'_, ()>, program: &GpuProgram) {
        if self.update() {
            // If the program is not already demanding a specific framerate, request a redraw
            if program.time.is_none() {
                program.viewport.request_redraw();
            }
        }

        if let Event::RedrawFrame(frame) = event {
            self.render(frame);
        } else if let Event::Winit(WinitEvent::WindowEvent { event, .. }) = event {
            match event {
                WindowEvent::Resized(size) => {
                    if let Some(state) = self.get_context_mut().try_use_global::<WindowSize>() {
                        let mut state = state.write();

                        state.width = size.width;
                        state.height = size.height;
                    }
                }

                WindowEvent::Moved(pos) => {
                    if let Some(state) = self.get_context_mut().try_use_global::<WindowPosition>() {
                        let mut state = state.write();

                        state.x = pos.x;
                        state.y = pos.y;
                    }
                }

                WindowEvent::ReceivedCharacter(c) => {
                    if let Some(state) = self.get_context_mut().try_use_global::<KeyboardInput>() {
                        state.write().0 = c;
                    }
                }

                WindowEvent::Focused(focused) => {
                    if let Some(state) = self.get_context_mut().try_use_global::<WindowFocus>() {
                        state.write().0 = focused;
                    }
                }

                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(state) = self.get_context_mut().try_use_global::<Keyboard>() {
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
                    if let Some(state) = self.get_context_mut().try_use_global::<Keyboard>() {
                        let mut state = state.write();

                        state.modifiers = unsafe { mem::transmute(modifiers) };
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(state) = self.get_context_mut().try_use_global::<Mouse>() {
                        let mut state = state.write();

                        match state.pos {
                            Some(ref mut pos) => {
                                pos.x = position.x;
                                pos.y = position.y;
                            }
                            None => {
                                state.pos = Some(XY {
                                    x: position.x,
                                    y: position.y,
                                });
                            }
                        }
                    }
                }

                WindowEvent::CursorLeft { .. } => {
                    if let Some(state) = self.get_context_mut().try_use_global::<Mouse>() {
                        let mut state = state.write();

                        state.pos = None;
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    if let Some(state) = self.get_context_mut().try_use_global::<Scroll>() {
                        let mut state = state.write();

                        match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                state.delta.x = x as f64;
                                state.delta.y = y as f64;
                            }
                            MouseScrollDelta::PixelDelta(position) => {
                                state.delta.x = position.x;
                                state.delta.y = position.y;
                            }
                        }
                    }
                }

                WindowEvent::MouseInput {
                    button,
                    state: value,
                    ..
                } => {
                    if let Some(state) = self.get_context_mut().try_use_global::<Mouse>() {
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
