use std::any::TypeId;

use agpu::{Binding, Buffer, Frame, GpuHandle, GpuProgram, Sampler, Texture, TextureFormat};
use agui::{context::Notify, widget::WidgetId, widgets::AppSettings, WidgetManager};
use downcast_rs::{impl_downcast, Downcast};

pub mod bounding;
pub mod clipping;
pub mod drawable;
pub mod text;

pub struct RenderContext {
    pub gpu: GpuHandle,

    pub layer_mask: Texture<agpu::D2>,
    pub layer_mask_sampler: Sampler,

    pub size: (u32, u32),

    pub app_settings: Notify<AppSettings>,
    pub app_settings_buffer: Buffer,
}

impl RenderContext {
    pub fn new(program: &GpuProgram, app_settings: Notify<AppSettings>) -> Self {
        let size = {
            let app_settings = app_settings.read();

            (app_settings.width as u32, app_settings.height as u32)
        };

        Self {
            gpu: GpuHandle::clone(&program.gpu),

            layer_mask: program
                .gpu
                .new_texture("agui layer mask")
                .with_format(TextureFormat::R32Uint)
                .allow_storage_binding()
                .allow_binding()
                .allow_copy_to()
                .allow_copy_from()
                .create_empty(size),
            layer_mask_sampler: program
                .gpu
                .new_sampler("agui drawable layer sampler")
                .create(),

            size,

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

        let size = (app_settings.width as u32, app_settings.height as u32);

        if self.size != size {
            self.size = size;

            self.app_settings_buffer
                .write_unchecked(&[app_settings.width, app_settings.height]);

            self.layer_mask.resize(self.size);
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
        layer: u32,
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
