use std::collections::HashSet;

use agpu::{Buffer, Frame, GpuHandle, GpuProgram, RenderPipeline};
use agui::{widget::WidgetID, WidgetManager};

pub struct RenderContext {
    pub gpu: GpuHandle,
    pub pipeline: RenderPipeline,
}

pub trait WidgetRenderPass {
    fn add(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: &WidgetID);

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager);

    fn remove(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: &WidgetID);

    fn render(&self, ctx: &RenderContext, frame: &mut Frame);
}

pub struct BasicRenderPass {
    widgets: HashSet<WidgetID>,

    buffer: Buffer,
}

impl BasicRenderPass {
    pub fn new(program: &GpuProgram) -> BasicRenderPass {
        BasicRenderPass {
            widgets: Default::default(),

            buffer: program
                .gpu
                .new_buffer("BasicRenderPass")
                .as_vertex_buffer()
                .create(&[0]),
        }
    }
}

impl WidgetRenderPass for BasicRenderPass {
    fn add(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: &WidgetID) {
        self.widgets.insert(*widget_id);
    }

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager) {}

    fn remove(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: &WidgetID) {
        self.widgets.remove(widget_id);
    }

    fn render(&self, ctx: &RenderContext, frame: &mut Frame) {
        frame
            .render_pass("basic render pass")
            .with_pipeline(&ctx.pipeline)
            .begin();
    }
}