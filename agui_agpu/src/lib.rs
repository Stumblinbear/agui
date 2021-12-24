use agpu::{Frame, GpuProgram};
use agui::{
    context::Ref,
    render::WidgetChanged,
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
            ctx: RenderContext::new(program),

            render_passes: BTreeMap::default(),
            render_pass_order: Vec::default(),
        }
    }

    pub fn new(program: &GpuProgram) -> WidgetRenderer {
        let renderer = WidgetRenderer::default(program);

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
            panic!("attempted to insert a duplicate render pass");
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

        pass.downcast_ref::<P>()
            .expect("failed to downcast render pass")
    }

    pub fn set_app_settings(&mut self, app_settings: Ref<AppSettings>) {
        self.ctx.set_app_settings(app_settings);
    }

    pub fn render(&mut self, mut frame: Frame) {
        self.ctx.update();

        for pass_type_id in &self.render_pass_order {
            self.render_passes
                .get(pass_type_id)
                .expect("render pass does not exist")
                .render(&self.ctx, &mut frame);
        }
    }
}
