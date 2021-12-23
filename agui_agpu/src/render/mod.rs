use agpu::{Frame, GpuHandle};
use agui::{render::WidgetChanged, WidgetManager};
use downcast_rs::{impl_downcast, Downcast};

pub mod bounding;
pub mod quad;

pub struct RenderContext {
    pub gpu: GpuHandle,
}

pub trait WidgetRenderPass: Downcast {
    fn added(&mut self, ctx: &RenderContext, manager: &WidgetManager, changed: &WidgetChanged);

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager, changed: &WidgetChanged);

    fn removed(&mut self, ctx: &RenderContext, manager: &WidgetManager, changed: &WidgetChanged);

    fn render(&self, ctx: &RenderContext, frame: &mut Frame);
}

impl_downcast!(WidgetRenderPass);
