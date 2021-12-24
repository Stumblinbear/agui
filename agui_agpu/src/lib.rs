use std::{any::TypeId, collections::BTreeMap};

use agpu::{Frame, GpuProgram};
use agui::{
    context::Ref,
    plugin::event::WidgetEvent,
    widget::{WidgetId, WidgetRef},
    widgets::{primitives::Quad, AppSettings},
    WidgetManager,
};
use render::{bounding::BoundingRenderPass, quad::QuadRenderPass, RenderContext, WidgetRenderPass};

pub mod render;

pub struct UI {
    manager: WidgetManager,
    events: Vec<WidgetEvent>,

    ctx: RenderContext,

    render_passes: BTreeMap<TypeId, Box<dyn WidgetRenderPass>>,
    render_pass_order: Vec<TypeId>,
}

impl UI {
    pub fn new(program: &GpuProgram) -> Self {
        let manager = WidgetManager::default();

        let app_settings = manager.get_context().set_global(AppSettings {
            width: program.viewport.inner_size().width as f32,
            height: program.viewport.inner_size().height as f32,
        });

        let ui = Self {
            manager,
            events: Vec::default(),

            ctx: RenderContext::new(program, Ref::clone(&app_settings)),

            render_passes: BTreeMap::default(),
            render_pass_order: Vec::default(),
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

    pub fn get_manager(&self) -> &WidgetManager {
        &self.manager
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

            true
        }else{
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
    }
}
