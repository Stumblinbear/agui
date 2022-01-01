use std::any::TypeId;

use agpu::{Binding, Buffer, Frame, GpuHandle, GpuProgram, Texture};
use agui::{context::Notify, unit::Rect, widget::WidgetId, widgets::AppSettings, WidgetManager};
use downcast_rs::{impl_downcast, Downcast};

pub mod bounding;
pub mod quad;
pub mod text;

pub struct RenderContext {
    pub gpu: GpuHandle,

    pub depth_buffer: Texture<agpu::D2>,

    pub app_settings: Notify<AppSettings>,
    pub app_settings_buffer: Buffer,
}

impl RenderContext {
    pub fn new(program: &GpuProgram, app_settings: Notify<AppSettings>) -> Self {
        Self {
            gpu: GpuHandle::clone(&program.gpu),

            depth_buffer: program
                .gpu
                .new_texture("agui_depth")
                .as_depth()
                .allow_binding()
                .allow_copy_to()
                .allow_copy_from()
                .create_empty({
                    let app_settings = app_settings.read();

                    (app_settings.width as u32, app_settings.height as u32)
                }),

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

        let new_size = (app_settings.width as u32, app_settings.height as u32);

        if self.depth_buffer.size != new_size {
            self.depth_buffer.resize(new_size);
        }
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
        z: f32,
    );

    fn removed(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    );

    fn update(&mut self, ctx: &RenderContext);

    fn render(&self, ctx: &RenderContext, frame: &mut Frame);
}

impl_downcast!(WidgetRenderPass);
