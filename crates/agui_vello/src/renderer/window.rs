use std::{error::Error, time::Instant};

use agui_core::unit::Size;
use agui_renderer::{BindRenderer, FrameNotifier, Renderer};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vello::{
    block_on_wgpu,
    util::{RenderContext, RenderSurface},
    AaConfig, AaSupport, RendererOptions,
};

use crate::view::VelloViewHandle;

#[derive(Clone)]
pub struct VelloWindowRenderer {
    view_handle: VelloViewHandle,
}

impl VelloWindowRenderer {
    pub fn new(view_handle: &VelloViewHandle) -> Self {
        Self {
            view_handle: view_handle.clone(),
        }
    }
}

impl<T> BindRenderer<T> for VelloWindowRenderer
where
    T: HasRawWindowHandle + HasRawDisplayHandle,
{
    async fn bind(
        self,
        target: &T,
        frame_notifier: FrameNotifier,
    ) -> Result<Box<dyn Renderer>, Box<dyn Error + Send + Sync>> {
        let mut render_context =
            RenderContext::new().map_err(|err| VelloBindError::Context(format!("{:?}", err)))?;

        let size = self
            .view_handle
            .with_scene(|scene| scene.size)
            .and_then(|size| if size.is_finite() { Some(size) } else { None })
            .unwrap_or_else(|| Size::new(16.0, 16.0));

        let now = Instant::now();

        let render_surface = render_context
            .create_surface(target, size.width as u32, size.height as u32)
            .await
            .map_err(|err| VelloBindError::Surface(format!("{:?}", err)))?;

        tracing::trace!("created render surface in {:?}", now.elapsed());

        let device_handle = &render_context.devices[render_surface.dev_id];

        let renderer = vello::Renderer::new(
            &device_handle.device,
            RendererOptions {
                surface_format: Some(render_surface.config.format),
                use_cpu: false,
                antialiasing_support: AaSupport::all(),
            },
        )
        .map_err(|err| VelloBindError::Renderer(format!("{:?}", err)))?;

        self.view_handle.set_frame_notifier(frame_notifier);

        Ok(Box::new(BoundVelloWindowRenderer {
            view_handle: self.view_handle,

            render_context,

            render_surface,
            renderer,
        }))
    }
}

struct BoundVelloWindowRenderer {
    view_handle: VelloViewHandle,

    render_context: RenderContext,

    render_surface: RenderSurface,
    renderer: vello::Renderer,
}

#[derive(Debug, thiserror::Error)]
pub enum VelloBindError {
    #[error("failed to create the render context")]
    Context(String),

    #[error("failed to create surface")]
    Surface(String),

    #[error("failed to create the renderer")]
    Renderer(String),
}

impl Renderer for BoundVelloWindowRenderer {
    fn render(&mut self) {
        tracing::trace!("VelloWindowRenderer::render");

        let render_context = &mut self.render_context;
        let render_surface = &mut self.render_surface;
        let renderer = &mut self.renderer;

        let device_handle = &render_context.devices[render_surface.dev_id];

        self.view_handle.with_scene(|scene| {
            let Some(size) = scene
                .size
                .and_then(|size| if size.is_finite() { Some(size) } else { None })
            else {
                tracing::warn!("scene has no size, skipping render");
                return;
            };

            if render_surface.config.width != size.width as u32
                || render_surface.config.height != size.height as u32
            {
                render_context.resize_surface(
                    render_surface,
                    size.width as u32,
                    size.height as u32,
                );
            }

            let surface_texture = render_surface
                .surface
                .get_current_texture()
                .expect("failed to get surface texture");

            let render_params = vello::RenderParams {
                base_color: vello::peniko::Color::BLACK,
                width: size.width as u32,
                height: size.height as u32,
                antialiasing_method: AaConfig::Area,
            };

            #[cfg(not(target_arch = "wasm32"))]
            {
                block_on_wgpu(
                    &device_handle.device,
                    renderer.render_to_surface_async(
                        &device_handle.device,
                        &device_handle.queue,
                        scene.as_ref(),
                        &surface_texture,
                        &render_params,
                    ),
                )
                .expect("failed to render to surface");
            }

            // Note: in the wasm case, we're currently not running the robust
            // pipeline, as it requires more async wiring for the readback.
            #[cfg(target_arch = "wasm32")]
            self.renderer
                .render_to_surface(
                    &device_handle.device,
                    &device_handle.queue,
                    scene.as_ref(),
                    &surface_texture,
                    &render_params,
                )
                .expect("failed to render to surface");

            surface_texture.present();
        });
    }
}
