use std::{any::TypeId, collections::HashMap};

use agpu::{Frame, GpuProgram, RenderPipeline};
use agui::{
    widget::{Quad, Widget, WidgetID},
    WidgetManager,
};
use render::{RenderQuad, RenderWidget};

pub mod render;

pub struct WidgetRenderer {
    pipeline: RenderPipeline,

    // I'm not a huge fan of the amount of indirection, here, but it lets us give tons of
    // control to the renderers
    renderers: HashMap<TypeId, Box<dyn Fn() -> Box<dyn RenderWidget>>>,
    renderer: HashMap<WidgetID, Box<dyn RenderWidget>>
}

impl agui::render::WidgetRenderer for WidgetRenderer {
    fn create(&mut self, manager: &WidgetManager, widget_id: WidgetID) {
        let type_id = (*manager.get(widget_id)).get_type_id();

        if let Some(builder) = self.renderers.get(&type_id) {
            self.renderer.insert(widget_id, builder());
        }
    }

    fn refresh(&mut self, manager: &WidgetManager) {
        // TODO: is it possible to limit the scope of layout refreshes?
        for (widget_id, renderer) in &self.renderer {
            renderer.draw(manager, *widget_id);
        }
    }

    fn remove(&mut self, _manager: &WidgetManager, widget_id: WidgetID) {
        self.renderer.remove(&widget_id);
    }
}

impl WidgetRenderer {
    pub fn without_primitives(program: &GpuProgram) -> WidgetRenderer {
        let pipeline = program.gpu.new_pipeline("agui_pipeline").create();

        WidgetRenderer {
            pipeline,

            renderers: Default::default(),

            renderer: Default::default(),
        }
    }

    pub fn new(program: &GpuProgram) -> WidgetRenderer {
        WidgetRenderer::without_primitives(program).init_renderer::<Quad, RenderQuad>()
    }

    pub fn init_renderer<W, R>(mut self) -> Self
    where
        W: Widget,
        R: RenderWidget + 'static + Default,
    {
        let type_id = TypeId::of::<W>();

        if self.renderers.insert(type_id, Box::new(|| Box::new(R::default()))).is_some() {
            panic!("attempted to overwrite an existing widget renderer");
        }

        self
    }

    pub fn render(&self, mut frame: Frame) {
        let mut pass = frame
            .render_pass("agui render pass")
            .with_pipeline(&self.pipeline)
            .begin();

        for renderer in self.renderer.values() {
            renderer.render(&mut pass);
        }
    }
}