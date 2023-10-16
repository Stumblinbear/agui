use std::{
    marker::PhantomData,
    sync::{mpsc, Arc},
};

use agui_core::unit::Size;
use agui_renderer::{RenderViewId, Renderer, ViewRenderer};
use futures::executor::block_on;
use parking_lot::Mutex;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rustc_hash::FxHashMap;
use vello::{util::RenderContext, RendererOptions, Scene};

use crate::{
    event::VelloPluginEvent,
    fonts::VelloFonts,
    render::{VelloViewRenderer, VelloViewRendererHandle},
};

pub struct VelloHandle<T> {
    pub(crate) phantom: PhantomData<T>,

    pub(crate) fonts: Arc<Mutex<VelloFonts>>,

    pub(crate) events_tx: mpsc::Sender<VelloPluginEvent>,
}

impl<W> Renderer for VelloHandle<W>
where
    W: HasRawWindowHandle + HasRawDisplayHandle,
{
    type Target = W;

    fn bind(
        &self,
        render_view_id: RenderViewId,
        target: &Self::Target,
        size: Size,
    ) -> Result<Arc<dyn ViewRenderer>, Box<dyn std::error::Error>> {
        let mut render_context = RenderContext::new()?;

        let surface = block_on(render_context.create_surface(
            &target,
            size.width.floor() as u32,
            size.height.floor() as u32,
        ))?;

        let device_handle = &render_context.devices[surface.dev_id];

        let renderer = vello::Renderer::new(
            &device_handle.device,
            &RendererOptions {
                surface_format: Some(surface.config.format),
                timestamp_period: device_handle.queue.get_timestamp_period(),
                use_cpu: false,
            },
        )?;

        let view_renderer_handle = Arc::new(VelloViewRendererHandle::new(VelloViewRenderer {
            fonts: Arc::clone(&self.fonts),

            render_context,

            render_view_id,

            surface,
            renderer,

            scene: Scene::new(),
            widgets: FxHashMap::default(),
        }));

        self.events_tx.send(VelloPluginEvent::ViewBind {
            render_view_id,
            renderer: Arc::clone(&view_renderer_handle),
        })?;

        Ok(view_renderer_handle)
    }
}
