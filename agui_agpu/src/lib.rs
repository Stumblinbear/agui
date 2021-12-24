use agpu::{Frame, GpuProgram};
use agui::{
    context::Ref,
    plugin::{event::WidgetEvent, WidgetPlugin},
    widgets::{primitives::Quad, AppSettings},
    WidgetManager,
};
use render::{bounding::BoundingRenderPass, quad::QuadRenderPass, RenderContext, WidgetRenderPass};
use std::{any::TypeId, collections::BTreeMap};

pub mod render;

pub struct WidgetRenderer {
    ctx: RenderContext,

    render_passes: BTreeMap<TypeId, Box<dyn WidgetRenderPass>>,
    render_pass_order: Vec<TypeId>,
}

impl WidgetPlugin for WidgetRenderer {
    fn on_event(&mut self, manager: &mut WidgetManager, event: &WidgetEvent) {
        match event {
            WidgetEvent::Added { type_id, widget_id } => {
                for pass in self.render_passes.values_mut() {
                    pass.added(&self.ctx, manager, type_id, widget_id);
                }
            },

            WidgetEvent::Layout {
                type_id,
                widget_id,
                rect,
            } => {
                println!("{}", widget_id);
                for pass in self.render_passes.values_mut() {
                    pass.layout(&self.ctx, manager, type_id, widget_id, rect);
                }
            },

            WidgetEvent::Removed { type_id, widget_id } => {
                for pass in self.render_passes.values_mut() {
                    pass.removed(&self.ctx, manager, type_id, widget_id);
                }
            },

            WidgetEvent::Updated => {
                self.ctx.update();
            },

            _ => { }
        }
    }
}

impl WidgetRenderer {
    pub fn default(program: &GpuProgram, app_settings: Ref<AppSettings>) -> WidgetRenderer {
        WidgetRenderer {
            ctx: RenderContext::new(program, app_settings),

            render_passes: BTreeMap::default(),
            render_pass_order: Vec::default(),
        }
    }

    pub fn new(program: &GpuProgram, app_settings: Ref<AppSettings>) -> WidgetRenderer {
        let renderer = WidgetRenderer::default(program, app_settings);

        let basic_pass = {
            let mut basic_pass = QuadRenderPass::new(program, &renderer.ctx);

            basic_pass.bind::<Quad>();

            basic_pass
        };

        let bounding_pass = BoundingRenderPass::new(program, &renderer.ctx);

        renderer.add_pass(basic_pass).add_pass(bounding_pass)
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

    pub fn render(&self, mut frame: Frame) {
        for pass_type_id in &self.render_pass_order {
            self.render_passes
                .get(pass_type_id)
                .expect("render pass does not exist")
                .render(&self.ctx, &mut frame);
        }
    }
}
