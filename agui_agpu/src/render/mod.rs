use agpu::{Frame, GpuHandle};
use agui::{WidgetID, WidgetManager};

pub mod quad;

pub struct RenderContext {
    pub gpu: GpuHandle,
}

pub trait WidgetRenderPass {
    fn add(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID);

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager);

    fn remove(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID);

    fn render(&self, ctx: &RenderContext, frame: &mut Frame);
}
