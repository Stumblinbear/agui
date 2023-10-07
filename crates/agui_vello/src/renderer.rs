use std::sync::Arc;

use agui_core::engine::{event::ElementEvent, Engine};
use agui_renderer::{RenderViewId, Renderer};
use futures::executor::block_on;
use parking_lot::Mutex;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rustc_hash::FxHashMap;
use vello::{util::RenderContext, RendererOptions, Scene};

use crate::{fonts::VelloFonts, surface::VelloSurface};

pub struct VelloRenderer<W> {
    phantom: std::marker::PhantomData<W>,

    render_context: RenderContext,

    fonts: Arc<Mutex<VelloFonts>>,

    surfaces: FxHashMap<RenderViewId, VelloSurface>,
}

impl<W> VelloRenderer<W> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::with_context(RenderContext::new()?))
    }

    pub fn with_context(context: RenderContext) -> Self {
        Self {
            phantom: std::marker::PhantomData,

            render_context: context,

            fonts: Arc::default(),

            surfaces: FxHashMap::default(),
        }
    }

    pub fn get_fonts(&self) -> &Arc<Mutex<VelloFonts>> {
        &self.fonts
    }
}

impl<W> Renderer<W> for VelloRenderer<W>
where
    W: HasRawWindowHandle + HasRawDisplayHandle,
{
    fn create_view(
        &mut self,
        engine: &Engine,
        render_view_id: RenderViewId,
        target: &W,
        width: u32,
        height: u32,
    ) {
        let surface = block_on(self.render_context.create_surface(&target, width, height)).unwrap();

        let device_handle = &self.render_context.devices[surface.dev_id];

        let renderer = vello::Renderer::new(
            &device_handle.device,
            &RendererOptions {
                surface_format: Some(surface.config.format),
                timestamp_period: device_handle.queue.get_timestamp_period(),
                use_cpu: false,
            },
        )
        .unwrap();

        let mut surface = VelloSurface {
            render_view_id,

            surface,
            renderer,

            scene: Scene::new(),
            widgets: FxHashMap::default(),
        };

        surface.init(engine, &mut self.fonts.lock());

        self.surfaces.insert(render_view_id, surface);
    }

    fn remove_view(&mut self, _: &Engine, render_view_id: RenderViewId) {
        self.surfaces.remove(&render_view_id);
    }

    fn resize(&mut self, _: &Engine, render_view_id: RenderViewId, width: u32, height: u32) {
        self.render_context.resize_surface(
            &mut self.surfaces.get_mut(&render_view_id).unwrap().surface,
            width,
            height,
        );
    }

    fn redraw(&mut self, engine: &Engine, render_view_id: RenderViewId, events: &[ElementEvent]) {
        self.surfaces.get_mut(&render_view_id).unwrap().redraw(
            engine,
            &mut self.fonts.lock(),
            events,
        );
    }

    fn render(&mut self, render_view_id: RenderViewId) {
        self.surfaces
            .get_mut(&render_view_id)
            .unwrap()
            .render(&self.render_context);
    }
}
