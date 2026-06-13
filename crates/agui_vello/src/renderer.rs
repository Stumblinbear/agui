use std::{num::NonZeroUsize, sync::Arc};

use agui_core::paint::compositing::CompositedFrame;
use agui_window::WindowRenderer;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use vello::{
    AaConfig, AaSupport, RenderParams, Renderer, RendererOptions,
    kurbo::Affine,
    peniko::Color,
    util::{RenderContext, RenderSurface},
    wgpu,
};

use crate::append_scene_with_transform;

/// A [`WindowRenderer`] that rasterizes a composed frame with vello and presents it to a wgpu surface.
///
/// It rasterizes the whole frame into one scene, so it presents content without a system compositor;
/// a frame that places a system-owned [`External`](agui_core::paint::compositing::CompositedNode::External)
/// surface needs a compositing renderer instead.
pub struct VelloWindowRenderer {
    context: RenderContext,
    /// One renderer per device; indexed by [`RenderSurface::dev_id`].
    renderers: Vec<Option<Renderer>>,
    surface: Option<RenderSurface<'static>>,
    scene: vello::Scene,
}

impl VelloWindowRenderer {
    pub fn new() -> Self {
        Self {
            context: RenderContext::new(),
            renderers: Vec::new(),
            surface: None,
            scene: vello::Scene::new(),
        }
    }
}

impl Default for VelloWindowRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowRenderer for VelloWindowRenderer {
    fn attach<W>(&mut self, window: Arc<W>, width: u32, height: u32)
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let surface = pollster::block_on(self.context.create_surface(
            window,
            width.max(1),
            height.max(1),
            wgpu::PresentMode::AutoVsync,
        ))
        .unwrap();

        self.renderers
            .resize_with(self.context.devices.len(), || None);
        self.renderers[surface.dev_id].get_or_insert_with(|| {
            Renderer::new(
                &self.context.devices[surface.dev_id].device,
                RendererOptions {
                    use_cpu: false,
                    antialiasing_support: AaSupport::area_only(),
                    num_init_threads: NonZeroUsize::new(1),
                    pipeline_cache: None,
                },
            )
            .unwrap()
        });

        self.surface = Some(surface);
    }

    fn resize(&mut self, width: u32, height: u32) {
        if let Some(surface) = self.surface.as_mut() {
            self.context
                .resize_surface(surface, width.max(1), height.max(1));
        }
    }

    fn present(&mut self, frame: &CompositedFrame, scale_factor: f64) {
        let Some(surface) = self.surface.as_ref() else {
            return;
        };

        let width = surface.config.width;
        let height = surface.config.height;

        self.scene.reset();
        append_scene_with_transform(
            &frame.rasterize(),
            &mut self.scene,
            Affine::scale(scale_factor),
        );

        let device = &self.context.devices[surface.dev_id];

        self.renderers[surface.dev_id]
            .as_mut()
            .unwrap()
            .render_to_texture(
                &device.device,
                &device.queue,
                &self.scene,
                &surface.target_view,
                &RenderParams {
                    base_color: Color::from_rgb8(30, 30, 30),
                    width,
                    height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .unwrap();

        let texture = surface.surface.get_current_texture().unwrap();
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        surface.blitter.copy(
            &device.device,
            &mut encoder,
            &surface.target_view,
            &texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );
        device.queue.submit([encoder.finish()]);
        texture.present();
    }
}
