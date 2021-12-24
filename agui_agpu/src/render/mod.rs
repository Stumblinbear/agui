use std::any::TypeId;

use agpu::{Binding, Buffer, Frame, GpuHandle, GpuProgram};
use agui::{context::Ref, unit::Rect, widget::WidgetId, widgets::AppSettings, WidgetManager};
use downcast_rs::{impl_downcast, Downcast};

pub mod bounding;
pub mod quad;

pub struct RenderContext {
    pub gpu: GpuHandle,

    pub app_settings: Ref<AppSettings>,
    pub app_settings_buffer: Buffer,
}

impl RenderContext {
    pub fn new(program: &GpuProgram, app_settings: Ref<AppSettings>) -> Self {
        Self {
            gpu: GpuHandle::clone(&program.gpu),

            app_settings,
            app_settings_buffer: program
                .gpu
                .new_buffer("AppSettings")
                .as_uniform_buffer()
                .allow_copy_to()
                .create(&[256.0, 256.0]),
        }
    }

    pub fn bind_app_settings(&self) -> Binding {
        self.app_settings_buffer.bind_uniform()
    }

    pub fn update(&mut self) {
        let app_settings = self.app_settings.read();

        self.app_settings_buffer
            .write_unchecked(&[app_settings.width, app_settings.height]);
    }
}

pub trait WidgetRenderPass: Downcast {
    fn added(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    );

    fn layout(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
        rect: &Rect,
    );

    fn removed(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    );

    fn render(&self, ctx: &RenderContext, frame: &mut Frame);
}

impl_downcast!(WidgetRenderPass);
