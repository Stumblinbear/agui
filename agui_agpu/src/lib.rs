use std::{any::TypeId, collections::BTreeMap, io, mem};

use agpu::{
    winit::winit::event::{
        ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    Event, Frame, GpuProgram,
};

use agui::{
    context::{Notify, WidgetContext},
    event::WidgetEvent,
    widget::{WidgetId, WidgetRef},
    widgets::{
        primitives::{FontId, Fonts, Quad},
        state::{
            keyboard::{KeyCode, KeyState, Keyboard, KeyboardInput},
            mouse::{Mouse, MouseButtonState, Scroll, XY},
            window::{WindowFocus, WindowPosition, WindowSize},
        },
        AppSettings,
    },
    WidgetManager,
};
use render::text::TextRenderPass;

pub mod render;

use self::render::{
    bounding::BoundingRenderPass, quad::QuadRenderPass, RenderContext, WidgetRenderPass,
};

pub struct UI {
    manager: WidgetManager<'static>,
    events: Vec<WidgetEvent>,

    text_pass: TextRenderPass,
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

            text_pass: TextRenderPass::new(program, &ctx),
            render_passes: BTreeMap::default(),
            render_pass_order: Vec::default(),

            ctx,
        };

        program.on_resize(move |_, w, h| {
            let mut app_settings = app_settings.write();

            app_settings.width = w as f32;
            app_settings.height = h as f32;
        });

        ui
    }

    pub fn with_default(program: &GpuProgram) -> Self {
        let ui = Self::new(program);

        let basic_pass = {
            let mut basic_pass = QuadRenderPass::new(program, &ui.ctx);

            basic_pass.bind::<Quad>();

            basic_pass
        };

        let bounding_pass = BoundingRenderPass::new(program, &ui.ctx);

        ui.add_pass(basic_pass).add_pass(bounding_pass)
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> FontId {
        let (font_id, font) = self
            .get_context()
            .get_global_or::<Fonts, _>(Fonts::default)
            .write()
            .load_bytes(bytes);

        self.text_pass.add_font(font);

        font_id
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<FontId> {
        match self
            .get_context()
            .get_global_or::<Fonts, _>(Fonts::default)
            .write()
            .load_file(filename)
        {
            Ok((font_id, font)) => {
                self.text_pass.add_font(font);

                Ok(font_id)
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
            .expect("cannot return a pass that has not been created");

        match pass.downcast_ref::<P>() {
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
            for event in self.events.drain(..) {
                match event {
                    WidgetEvent::Spawned { type_id, widget_id } => {
                        for pass in self.render_passes.values_mut() {
                            pass.added(&self.ctx, &self.manager, &type_id, &widget_id);
                        }
                    }

                    WidgetEvent::Layout {
                        type_id,
                        widget_id,
                        rect,
                    } => {
                        for pass in self.render_passes.values_mut() {
                            pass.layout(&self.ctx, &self.manager, &type_id, &widget_id, &rect);
                        }
                    }

                    WidgetEvent::Destroyed { type_id, widget_id } => {
                        for pass in self.render_passes.values_mut() {
                            pass.removed(&self.ctx, &self.manager, &type_id, &widget_id);
                        }
                    }

                    _ => {}
                }
            }

            self.ctx.update();

            for pass_type_id in &self.render_pass_order {
                self.render_passes
                    .get_mut(pass_type_id)
                    .expect("render pass does not exist")
                    .update(&self.ctx);
            }

            self.text_pass.update(&self.ctx);

            true
        } else {
            false
        }
    }

    pub fn render(&self, mut frame: Frame) {
        for pass_type_id in &self.render_pass_order {
            self.render_passes
                .get(pass_type_id)
                .expect("render pass does not exist")
                .render(&self.ctx, &mut frame);
        }

        self.text_pass.render(&self.ctx, &mut frame);
    }

    pub fn run(mut self, program: GpuProgram) -> Result<(), agpu::BoxError> {
        let pipeline = program.gpu.new_pipeline("render pipeline").create();

        program.run(move |event, _, _| {
            if let Event::RedrawFrame(mut frame) = event {
                if self.update() {
                    // self.manager.print_tree();
                }

                frame
                    .render_pass_cleared("ui draw", 0x101010FF)
                    .with_pipeline(&pipeline)
                    .begin();

                self.render(frame);
            } else if let Event::Winit(WinitEvent::WindowEvent { event, .. }) = event {
                match event {
                    WindowEvent::Resized(size) => {
                        if let Some(state) = self.get_context().get_global::<WindowSize>() {
                            let mut state = state.write();

                            state.width = size.width;
                            state.height = size.height;
                        }
                    }

                    WindowEvent::Moved(pos) => {
                        if let Some(state) = self.get_context().get_global::<WindowPosition>() {
                            let mut state = state.write();

                            state.x = pos.x;
                            state.y = pos.y;
                        }
                    }

                    WindowEvent::ReceivedCharacter(c) => {
                        if let Some(state) = self.get_context().get_global::<KeyboardInput>() {
                            state.write().0 = c;
                        }
                    }

                    WindowEvent::Focused(focused) => {
                        if let Some(state) = self.get_context().get_global::<WindowFocus>() {
                            state.write().0 = focused;
                        }
                    }

                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(state) = self.get_context().get_global::<Keyboard>() {
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
                        if let Some(state) = self.get_context().get_global::<Keyboard>() {
                            let mut state = state.write();

                            state.modifiers = unsafe { mem::transmute(modifiers) };
                        }
                    }

                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some(state) = self.get_context().get_global::<Mouse>() {
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
                        if let Some(state) = self.get_context().get_global::<Mouse>() {
                            let mut state = state.write();

                            state.pos = None;
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        if let Some(state) = self.get_context().get_global::<Scroll>() {
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
                        if let Some(state) = self.get_context().get_global::<Mouse>() {
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
