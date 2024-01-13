use std::hash::BuildHasherDefault;

use agui_core::render::RenderObjectId;
use agui_renderer::RenderWindow;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;
use vello::{
    block_on_wgpu,
    util::{RenderContext, RenderSurface},
    RendererOptions, Scene,
};

use crate::{
    render::VelloRenderObject,
    view::{VelloView, VelloViewHandle},
};

mod sealed {
    pub trait VelloWindowRendererState {}
}

impl sealed::VelloWindowRendererState for () {}

pub struct Attached {
    render_context: RenderContext,

    render_surface: RenderSurface,
    renderer: vello::Renderer,

    scene: Scene,
    render_objects:
        SparseSecondaryMap<RenderObjectId, VelloRenderObject, BuildHasherDefault<FxHasher>>,
}

impl sealed::VelloWindowRendererState for Attached {}

pub struct VelloWindowRenderer<S>
where
    S: sealed::VelloWindowRendererState,
{
    view_handle: VelloViewHandle,

    state: S,
}

impl Clone for VelloWindowRenderer<()> {
    fn clone(&self) -> Self {
        Self {
            view_handle: self.view_handle.clone(),

            state: (),
        }
    }
}

impl VelloWindowRenderer<()> {
    pub fn new(view: &VelloView) -> Self {
        Self {
            view_handle: view.handle(),

            state: (),
        }
    }

    pub fn attach<W>(
        &self,
        window: &W,
    ) -> Result<VelloWindowRenderer<Attached>, Box<dyn std::error::Error>>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let mut render_context = RenderContext::new()?;

        let render_surface =
            futures::executor::block_on(render_context.create_surface(window, 1_u32, 1_u32))?;

        let device_handle = &render_context.devices[render_surface.dev_id];

        let renderer = vello::Renderer::new(
            &device_handle.device,
            &RendererOptions {
                surface_format: Some(render_surface.config.format),
                timestamp_period: device_handle.queue.get_timestamp_period(),
                use_cpu: false,
            },
        )?;

        Ok(VelloWindowRenderer {
            view_handle: self.view_handle.clone(),

            state: Attached {
                render_context,

                render_surface,
                renderer,

                scene: Scene::new(),
                render_objects: SparseSecondaryMap::default(),
            },
        })
    }
}

impl RenderWindow for VelloWindowRenderer<Attached> {
    fn render(&mut self) {
        tracing::debug!("VelloWindowRenderer::render");

        let render_context = &mut self.state.render_context;
        let render_surface = &mut self.state.render_surface;
        let renderer = &mut self.state.renderer;
        let scene = &mut self.state.scene;

        let width = render_surface.config.width;
        let height = render_surface.config.height;
        let device_handle = &render_context.devices[render_surface.dev_id];

        let surface_texture = render_surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        let render_params = vello::RenderParams {
            base_color: vello::peniko::Color::BLACK,
            width,
            height,
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            block_on_wgpu(
                &device_handle.device,
                renderer.render_to_surface_async(
                    &device_handle.device,
                    &device_handle.queue,
                    scene,
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
                &scene,
                &surface_texture,
                &render_params,
            )
            .expect("failed to render to surface");

        surface_texture.present();
    }
}
