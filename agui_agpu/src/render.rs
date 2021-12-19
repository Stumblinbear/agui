use std::collections::HashSet;

use agpu::{Buffer, Frame, GpuHandle, GpuProgram, RenderPipeline};
use agui::{widget::WidgetID, WidgetManager};

pub struct RenderContext {
    pub gpu: GpuHandle,
    pub pipeline: RenderPipeline,
}

pub trait WidgetRenderPass {
    fn add(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID);

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager);

    fn remove(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID);

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
                .allow_copy_to()
                .create_uninit(std::mem::size_of::<[f32; 4]>() as u64),
        }
    }
}

impl WidgetRenderPass for BasicRenderPass {
    fn add(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID) {
        let rect = manager
            .get_rect(&widget_id)
            .expect("widget added to render pass does not have a rect");

        dbg!(&rect);

        self.buffer
            .write_unchecked(&[rect.x, rect.y, rect.width, rect.height]);

        self.widgets.insert(widget_id);
    }

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager) {}

    fn remove(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID) {
        self.widgets.remove(&widget_id);
    }

    fn render(&self, ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("basic render pass")
            .with_pipeline(&ctx.pipeline)
            .begin();

        r.set_vertex_buffer(0, self.buffer.slice(..))
            .draw(0..6, 0..1);
    }
}
