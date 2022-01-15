use std::{any::TypeId, collections::BTreeMap, io, mem};

use agpu::{
    winit::winit::event::{
        ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    Event, Frame, GpuProgram, RenderPipeline,
};

use agui::{
    context::{Notify, WidgetContext},
    event::WidgetEvent,
    widget::{WidgetId, WidgetRef},
    widgets::{
        primitives::{FontDescriptor, Fonts},
        state::{
            keyboard::{KeyCode, KeyState, Keyboard, KeyboardInput},
            mouse::{Mouse, MouseButtonState, Scroll, XY},
            window::{WindowFocus, WindowPosition, WindowSize},
        },
        AppSettings,
    },
    WidgetManager,
};
use render::clipping::ClippingRenderPass;

pub mod render;

use self::render::{
    drawable::DrawableRenderPass, text::TextRenderPass, RenderContext, WidgetRenderPass,
};

pub struct UI {
    manager: WidgetManager<'static>,
    events: Vec<WidgetEvent>,

    pipeline: RenderPipeline,

    clipping_pass: ClippingRenderPass,

    render_passes: BTreeMap<TypeId, Box<dyn WidgetRenderPass>>,
    render_pass_order: Vec<TypeId>,

    ctx: RenderContext,
}

impl UI {
    pub fn new(program: &GpuProgram) -> Self {
        let manager = WidgetManager::default();

        let app_settings = manager.get_context().init_global(|| AppSettings {
            width: program.viewport.inner_size().width as f32,
            height: program.viewport.inner_size().height as f32,
        });

        let ctx = RenderContext::new(program, Notify::clone(&app_settings));

        let ui = Self {
            manager,
            events: Vec::default(),

            pipeline: program.gpu.new_pipeline("agui clear pipeline").create(),

            clipping_pass: ClippingRenderPass::new(program, &ctx),

            render_passes: BTreeMap::default(),
            render_pass_order: Vec::default(),

            ctx,
        };

        program.on_resize(move |_, w, h| {
            if w == 0 && h == 0 {
                return;
            }

            let mut app_settings = app_settings.write();

            app_settings.width = w as f32;
            app_settings.height = h as f32;
        });

        ui
    }

    pub fn with_default(program: &GpuProgram) -> Self {
        let ui = Self::new(program);

        let drawable_pass = DrawableRenderPass::new(program, &ui.ctx);
        let text_pass = TextRenderPass::new(program, &ui.ctx);

        ui.add_pass(drawable_pass).add_pass(text_pass)
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> FontDescriptor {
        let (font, font_arc) = self
            .get_context()
            .use_global::<Fonts, _>(Fonts::default)
            .write()
            .load_bytes(bytes);

        self.get_pass_mut::<TextRenderPass>().add_font(font_arc);

        font
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<FontDescriptor> {
        match self
            .get_context()
            .use_global::<Fonts, _>(Fonts::default)
            .write()
            .load_file(filename)
        {
            Ok((font, font_arc)) => {
                self.get_pass_mut::<TextRenderPass>().add_font(font_arc);

                Ok(font)
            }
            Err(err) => Err(err),
        }
    }

    pub fn add_pass<P>(mut self, pass: P) -> Self
    where
        P: WidgetRenderPass + 'static,
    {
        let pass_type_id = TypeId::of::<P>();

        if self
            .render_passes
            .insert(pass_type_id, Box::new(pass))
            .is_some()
        {
            log::warn!("inserted a duplicate render pass, overwriting");
        }

        self.render_pass_order.push(pass_type_id);

        self
    }

    pub fn get_pass<P>(&self) -> &P
    where
        P: WidgetRenderPass + 'static,
    {
        let pass = self
            .render_passes
            .get(&TypeId::of::<P>())
            .unwrap_or_else(|| panic!("render pass not added: {}", std::any::type_name::<P>()));

        match pass.downcast_ref::<P>() {
            Some(pass) => pass,
            None => unreachable!(),
        }
    }

    pub fn get_pass_mut<P>(&mut self) -> &mut P
    where
        P: WidgetRenderPass + 'static,
    {
        let pass = self
            .render_passes
            .get_mut(&TypeId::of::<P>())
            .unwrap_or_else(|| panic!("render pass not added: {}", std::any::type_name::<P>()));

        match pass.downcast_mut::<P>() {
            Some(pass) => pass,
            None => unreachable!(),
        }
    }

    pub fn get_context(&self) -> &WidgetContext<'static> {
        self.manager.get_context()
    }

    pub fn set_root(&mut self, widget: WidgetRef) {
        self.manager.add(None, widget);
    }

    pub fn add(&mut self, parent_id: Option<WidgetId>, widget: WidgetRef) {
        self.manager.add(parent_id, widget);
    }

    pub fn remove(&mut self, widget_id: WidgetId) {
        self.manager.remove(widget_id);
    }

    pub fn update(&mut self) -> bool {
        self.manager.update(&mut self.events);

        if !self.events.is_empty() {
            self.ctx.update();

            self.clipping_pass.update(&self.ctx, &self.manager, &self.events);

            for pass_type_id in &self.render_pass_order {
                self.render_passes
                    .get_mut(pass_type_id)
                    .expect("render pass does not exist")
                    .update(&self.ctx, &self.manager, &self.events);
            }

            self.events.clear();

            true
        } else {
            false
        }
    }

    pub fn render(&self, mut frame: Frame) {
        // We complete rendering by first clearing the screen, then creating the depth buffer based on
        // clipping masks, before finally rendering the actual widgets through the added render passes.
        frame
            .render_pass_cleared("agui clear pass", 0x101010FF)
            .with_pipeline(&self.pipeline)
            .begin();

        self.clipping_pass.render(&self.ctx, &mut frame);

        for pass_type_id in &self.render_pass_order {
            self.render_passes
                .get(pass_type_id)
                .expect("render pass does not exist")
                .render(&self.ctx, &mut frame);
        }
    }

    pub fn run(mut self, program: GpuProgram) -> Result<(), agpu::BoxError> {
        let mut i = 0;

        program.run(move |event, program, _, _| {
            if self.update() {
                self.manager.print_tree();

                // If the program is not already demanding a specific framerate, request a redraw
                if program.time.is_none() {
                    program.viewport.request_redraw();
                }
            }

            if let Event::RedrawFrame(frame) = event {
                i += 1;

                if i % 100 == 0 {
                    let now = std::time::Instant::now();

                    self.render(frame);

                    let then = std::time::Instant::now();

                    let frame_time = (then - now).as_secs_f64();

                    program.viewport.window.set_title(&format!(
                        "fps: {:.2} frame_time: {:.2}",
                        1.0 / frame_time,
                        frame_time * 1000.0
                    ));
                } else {
                    self.render(frame);
                }
            } else if let Event::Winit(WinitEvent::WindowEvent { event, .. }) = event {
                match event {
                    WindowEvent::Resized(size) => {
                        if let Some(state) = self.get_context().try_use_global::<WindowSize>() {
                            let mut state = state.write();

                            state.width = size.width;
                            state.height = size.height;
                        }
                    }

                    WindowEvent::Moved(pos) => {
                        if let Some(state) = self.get_context().try_use_global::<WindowPosition>() {
                            let mut state = state.write();

                            state.x = pos.x;
                            state.y = pos.y;
                        }
                    }

                    WindowEvent::ReceivedCharacter(c) => {
                        if let Some(state) = self.get_context().try_use_global::<KeyboardInput>() {
                            state.write().0 = c;
                        }
                    }

                    WindowEvent::Focused(focused) => {
                        if let Some(state) = self.get_context().try_use_global::<WindowFocus>() {
                            state.write().0 = focused;
                        }
                    }

                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(state) = self.get_context().try_use_global::<Keyboard>() {
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
                        if let Some(state) = self.get_context().try_use_global::<Keyboard>() {
                            let mut state = state.write();

                            state.modifiers = unsafe { mem::transmute(modifiers) };
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(state) = self.get_context().try_use_global::<Mouse>() {
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
                        if let Some(state) = self.get_context().try_use_global::<Mouse>() {
                            let mut state = state.write();

                            state.pos = None;
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        if let Some(state) = self.get_context().try_use_global::<Scroll>() {
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
                        if let Some(state) = self.get_context().try_use_global::<Mouse>() {
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
        });
    }
}
