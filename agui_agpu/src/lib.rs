use agpu::{Frame, GpuProgram};
use agui::{render::WidgetChanged, widgets::primitives::Quad, WidgetManager};
use render::{bounding::BoundingRenderPass, quad::QuadRenderPass, RenderContext, WidgetRenderPass};
use std::{any::TypeId, collections::HashMap};

pub mod render;

pub struct WidgetRenderer {
    ctx: RenderContext,

    render_passes: HashMap<TypeId, Box<dyn WidgetRenderPass>>,
}

impl agui::render::WidgetRenderer for WidgetRenderer {
    fn added(&mut self, manager: &WidgetManager, changed: WidgetChanged) {
        for pass in self.render_passes.values_mut() {
            pass.added(&self.ctx, manager, &changed);
        }
    }

    fn refresh(&mut self, manager: &WidgetManager, changed: WidgetChanged) {
        for pass in self.render_passes.values_mut() {
            pass.refresh(&self.ctx, manager, &changed);
        }
    }

    fn removed(&mut self, manager: &WidgetManager, changed: WidgetChanged) {
        for pass in self.render_passes.values_mut() {
            pass.removed(&self.ctx, manager, &changed);
        }
    }
}

impl WidgetRenderer {
    pub fn default(program: &GpuProgram) -> WidgetRenderer {
        WidgetRenderer {
            ctx: RenderContext {
                gpu: program.gpu.clone(),
            },

            render_passes: Default::default(),
        }
    }

    pub fn new(program: &GpuProgram) -> WidgetRenderer {
        let mut basic_pass = QuadRenderPass::new(program);

        basic_pass.bind::<Quad>();

        WidgetRenderer::default(program).add_pass(basic_pass)
        // .add_pass(BoundingRenderPass::new(program))
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
            panic!("attempted to insert a duplicate render pass");
        }

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

        pass.downcast_ref::<P>()
            .expect("failed to downcast render pass")
    }

    pub fn render(&self, mut frame: Frame) {
        for renderer in self.render_passes.values() {
            renderer.render(&self.ctx, &mut frame);
        }
    }
}
