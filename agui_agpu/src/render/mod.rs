use agpu::{Binding, Buffer, Frame, GpuHandle, GpuProgram};
use agui::{context::Ref, render::WidgetChanged, widgets::AppSettings, WidgetManager};
use downcast_rs::{impl_downcast, Downcast};

pub mod bounding;
pub mod quad;

pub struct RenderContext {
    pub gpu: GpuHandle,

    pub app_settings: Option<Ref<AppSettings>>,
    pub app_settings_buffer: Buffer,
}

impl RenderContext {
    pub fn new(program: &GpuProgram) -> Self {
        Self {
            gpu: GpuHandle::clone(&program.gpu),

            app_settings: None,
            app_settings_buffer: program
                .gpu
                .new_buffer("AppSettings")
                .as_uniform_buffer()
                .allow_copy_to()
                .create(&[256.0, 256.0]),
        }
    }

    pub fn set_app_settings(&mut self, app_settings: Ref<AppSettings>) {
        self.app_settings = Some(app_settings);
    }

    pub fn bind_app_settings(&self) -> Binding {
        self.app_settings_buffer.bind_uniform()
    }

    pub fn update(&mut self) {
        if let Some(app_settings) = &self.app_settings {
            let app_settings = app_settings.read();

            self.app_settings_buffer
                .write_unchecked(&[app_settings.width, app_settings.height]);
        }
    }
}

pub trait WidgetRenderPass: Downcast {
    fn added(&mut self, ctx: &RenderContext, manager: &WidgetManager, changed: &WidgetChanged);

    fn refresh(&mut self, ctx: &RenderContext, manager: &WidgetManager, changed: &WidgetChanged);

    fn removed(&mut self, ctx: &RenderContext, manager: &WidgetManager, changed: &WidgetChanged);

    fn render(&self, ctx: &RenderContext, frame: &mut Frame);
}

impl_downcast!(WidgetRenderPass);
