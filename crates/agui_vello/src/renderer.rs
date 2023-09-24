use std::rc::Rc;

use agui_core::{
    engine::{event::ElementEvent, Engine},
    plugin::Plugin,
    render::{renderer::Renderer, RenderViewId},
    unit::Font,
    widget::{IntoWidget, Widget},
};
use agui_macros::build;
use agui_primitives::text::layout_controller::TextLayoutController;
use futures::executor::block_on;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rustc_hash::FxHashMap;
use vello::{fello::raw::FontRef, util::RenderContext, RendererOptions, Scene};

use crate::{fonts::VelloFonts, surface::VelloSurface, text_layout::VelloTextLayoutDelegate};

pub struct VelloRenderer<'r, W> {
    phantom: std::marker::PhantomData<W>,

    render_view: RenderContext,

    fonts: VelloFonts<'r>,

    surfaces: FxHashMap<RenderViewId, VelloSurface>,
}

impl<W> Plugin for VelloRenderer<'_, W> {
    fn build<T: IntoWidget>(&self, child: impl Into<Option<T>>) -> Widget {
        build! {
            <TextLayoutController> {
                delegate: Rc::new(VelloTextLayoutDelegate {
                    default_font: FontRef::new(include_bytes!(
                        "../../../examples/fonts/DejaVuSans.ttf"
                    ))
                    .unwrap(),
                }),

                child: child.into().map(IntoWidget::into_widget),
            }
        }
    }
}

impl<'r, W> VelloRenderer<'r, W> {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self::with_context(RenderContext::new()?))
    }

    pub fn with_context(context: RenderContext) -> Self {
        Self {
            phantom: std::marker::PhantomData,

            render_view: context,

            fonts: VelloFonts::default(),

            surfaces: FxHashMap::default(),
        }
    }

    pub fn with_fonts(mut self, fonts: impl IntoIterator<Item = FontRef<'r>>) -> Self {
        for font in fonts {
            self.fonts.add_font(font);
        }

        self
    }

    pub fn add_font(&mut self, font: FontRef<'r>) -> Font {
        self.fonts.add_font(font)
    }
}

impl<W> Renderer for VelloRenderer<'_, W>
where
    W: HasRawWindowHandle + HasRawDisplayHandle,
{
    type Target = W;

    fn create_context(
        &mut self,
        engine: &Engine,
        render_view_id: RenderViewId,
        target: &Self::Target,
        width: u32,
        height: u32,
    ) {
        let surface = block_on(self.render_view.create_surface(&target, width, height)).unwrap();

        let device_handle = &self.render_view.devices[surface.dev_id];

        let renderer = vello::Renderer::new(
            &device_handle.device,
            &RendererOptions {
                surface_format: Some(surface.config.format),
                timestamp_period: device_handle.queue.get_timestamp_period(),
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

        surface.init(engine, &mut self.fonts);

        self.surfaces.insert(render_view_id, surface);
    }

    fn remove_context(&mut self, _: &Engine, render_view_id: RenderViewId) {
        self.surfaces.remove(&render_view_id);
    }

    fn resize(&mut self, _: &Engine, render_view_id: RenderViewId, width: u32, height: u32) {
        self.render_view.resize_surface(
            &mut self.surfaces.get_mut(&render_view_id).unwrap().surface,
            width,
            height,
        );
    }

    fn redraw(&mut self, engine: &Engine, render_view_id: RenderViewId, events: &[ElementEvent]) {
        self.surfaces
            .get_mut(&render_view_id)
            .unwrap()
            .redraw(engine, &mut self.fonts, events);
    }

    fn render(&mut self, render_view_id: RenderViewId) {
        self.surfaces
            .get_mut(&render_view_id)
            .unwrap()
            .render(&self.render_view);
    }
}
