use std::{any::TypeId, collections::HashMap};

use agpu::{Frame, GpuProgram};
use agui::{
    widget::{Widget, WidgetID, Quad},
    WidgetManager,
};
use render::{BasicRenderPass, RenderContext, WidgetRenderPass};

pub mod render;

pub struct WidgetRenderer {
    ctx: RenderContext,

    render_passes: HashMap<TypeId, Box<dyn WidgetRenderPass>>,
    bound_render_pass: HashMap<TypeId, TypeId>,

    widget_renderer: HashMap<WidgetID, TypeId>,
}

impl agui::render::WidgetRenderer for WidgetRenderer {
    fn create(&mut self, manager: &WidgetManager, widget_id: WidgetID) {
        let widget_type_id = (*manager.get(widget_id)).get_type_id();

        if let Some(pass_type_id) = self.bound_render_pass.get(&widget_type_id) {
            self.render_passes
                .get_mut(pass_type_id)
                .expect("impossible render pass access")
                .add(&self.ctx, manager, &widget_id);
        }
    }

    fn refresh(&mut self, manager: &WidgetManager) {
        // TODO: is it possible to limit the scope of layout refreshes?
        for (_, pass_type_id) in &self.bound_render_pass {
            self.render_passes
                .get_mut(pass_type_id)
                .expect("impossible render pass access")
                .refresh(&self.ctx, manager);
        }
    }

    fn remove(&mut self, manager: &WidgetManager, widget_id: WidgetID) {
        if let Some(pass_type_id) = self.widget_renderer.remove(&widget_id) {
            self.render_passes
                .get_mut(&pass_type_id)
                .expect("impossible render pass access")
                .remove(&self.ctx, manager, &widget_id);
        }
    }
}

impl WidgetRenderer {
    pub fn without_primitives(program: &GpuProgram) -> WidgetRenderer {
        let pipeline = program
            .gpu
            .new_pipeline("agui_pipeline")
            .with_vertex_fragment(include_bytes!("shader/ui.wgsl"))
            .create();

        WidgetRenderer {
            ctx: RenderContext {
                gpu: program.gpu.clone(),
                pipeline,
            },

            render_passes: Default::default(),
            bound_render_pass: Default::default(),

            widget_renderer: Default::default(),
        }
    }

    pub fn new(program: &GpuProgram) -> WidgetRenderer {
        let basic_pass = BasicRenderPass::new(program);

        WidgetRenderer::without_primitives(program)
            .add_render_pass(basic_pass)
            .bind_widget_pass::<BasicRenderPass, Quad>()
    }

    pub fn init_render_pass<P>(self) -> Self
    where
        P: WidgetRenderPass + Default + 'static,
    {
        self.add_render_pass(P::default())
    }

    pub fn add_render_pass<P>(mut self, pass: P) -> Self
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

    pub fn bind_widget_pass<P, W>(mut self) -> Self
    where
        P: WidgetRenderPass + 'static,
        W: Widget + 'static,
    {
        let pass_type_id = TypeId::of::<P>();
        let widget_type_id = TypeId::of::<W>();

        self.bound_render_pass.insert(widget_type_id, pass_type_id);

        self
    }

    pub fn render(&self, mut frame: Frame) {
        frame
            .render_pass_cleared("agui render pass", 0x000000)
            .with_pipeline(&self.ctx.pipeline)
            .begin()
            .draw_triangle();

        for (_, renderer) in &self.render_passes {
            renderer.render(&self.ctx, &mut frame);
        }
    }
}
