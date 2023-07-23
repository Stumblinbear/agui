use std::time::Instant;

use agui::{
    element::ElementId,
    manager::{events::ElementEvent, WidgetManager},
    unit::Offset,
};
use fnv::FnvHashMap;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vello::{
    block_on_wgpu,
    glyph::GlyphContext,
    kurbo::{Affine, Vec2},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    Renderer, RendererOptions, Scene, SceneBuilder,
};

use crate::element::RenderElement;

pub(crate) struct RenderManager {
    surface: RenderSurface,

    render_cx: RenderContext,
    renderer: Renderer,

    glyph_cx: GlyphContext,

    scene: Scene,

    widgets: FnvHashMap<ElementId, RenderElement>,
}

impl RenderManager {
    pub async fn new<W>(window: &W, width: u32, height: u32) -> Self
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let mut render_cx = RenderContext::new().unwrap();

        let surface = render_cx
            .create_surface(&window, width, height)
            .await
            .unwrap();

        let device_handle = &render_cx.devices[surface.dev_id];

        let renderer = Renderer::new(
            &device_handle.device,
            &RendererOptions {
                surface_format: Some(surface.config.format),
                timestamp_period: device_handle.queue.get_timestamp_period(),
            },
        )
        .unwrap();

        Self {
            surface,

            render_cx,
            renderer,

            glyph_cx: GlyphContext::new(),

            scene: Scene::new(),

            widgets: FnvHashMap::default(),
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.render_cx
            .resize_surface(&mut self.surface, width, height);
    }

    pub fn redraw(&mut self, manager: &WidgetManager, events: &[ElementEvent]) {
        let now = Instant::now();

        for event in events {
            match event {
                ElementEvent::Spawned {
                    parent_id,
                    element_id,
                } => {
                    self.widgets.insert(
                        *element_id,
                        RenderElement {
                            head_target: parent_id.and_then(|parent_id| {
                                let parent = self
                                    .widgets
                                    .get(&parent_id)
                                    .expect("render element spawned to a non-existent parent");

                                if parent.canvas.tail.is_some() {
                                    Some(parent_id)
                                } else {
                                    parent.head_target
                                }
                            }),

                            offset: Offset::ZERO,

                            ..RenderElement::default()
                        },
                    );
                }

                ElementEvent::Rebuilt { .. } => {}

                ElementEvent::Reparent {
                    parent_id,
                    element_id,
                } => {
                    let new_head_target = parent_id.and_then(|parent_id| {
                        let parent = self
                            .widgets
                            .get(&parent_id)
                            .expect("render element spawned to a non-existent parent");

                        if parent.canvas.tail.is_some() {
                            Some(parent_id)
                        } else {
                            parent.head_target
                        }
                    });

                    let element = self
                        .widgets
                        .get_mut(element_id)
                        .expect("reparented render element not found");

                    element.head_target = new_head_target;
                }

                ElementEvent::Destroyed { element_id } => {
                    self.widgets.remove(element_id);
                }

                ElementEvent::Layout { element_id } => {
                    self.update_element(manager, *element_id);
                }

                ElementEvent::Draw { element_id } => {
                    self.update_element(manager, *element_id);
                }

                _ => todo!(),
            }
        }

        let mut builder = SceneBuilder::for_scene(&mut self.scene);

        let mut element_stack = Vec::<(usize, ElementId, Affine)>::new();

        for element_id in manager.get_tree().iter_down() {
            let element = self.widgets.get(&element_id).unwrap();

            let element_depth = manager.get_tree().get_depth(element_id).unwrap();

            // End any elements in the stack that are at the same level or deeper than this one
            while let Some((element_id, transform)) = element_stack
                .last()
                .filter(|(depth, ..)| *depth >= element_depth)
                .map(|(_, element_id, transform)| (*element_id, transform))
            {
                let element = self.widgets.get(&element_id).unwrap();

                element.canvas.end(*transform, &mut builder);

                element_stack.pop();
            }

            let transform = element_stack
                .last()
                .map(|entry| entry.2)
                .unwrap_or(Affine::IDENTITY);

            let offset = element.offset;

            let transform =
                transform * Affine::translate(Vec2::new(offset.x as f64, offset.y as f64));

            element.canvas.begin(transform, &mut builder);

            element_stack.push((element_depth, element_id, transform));
        }

        // End any remaining elements in the stack
        while let Some((_, element_id, transform)) = element_stack.pop() {
            let element = self.widgets.get(&element_id).unwrap();

            element.canvas.end(transform, &mut builder);
        }

        tracing::info!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    fn update_element(&mut self, manager: &WidgetManager, element_id: ElementId) {
        let render_element = self
            .widgets
            .get_mut(&element_id)
            .expect("drawn render element not found");

        let widget_element = manager.get_tree().get(element_id).unwrap();

        let canvas = widget_element.paint();

        render_element.offset = widget_element.get_offset();

        render_element.canvas.update(&mut self.glyph_cx, canvas);

        // if let Some(canvas) = canvas {
        //     let pos = Point::from(widget_element.get_rect().cloned().unwrap());

        //     // If we have or are drawing to the target layer, mark it dirty
        //     // if !canvas.head.is_empty() || render_element.head.is_some() {
        //     //     dirty_layers.extend(render_element.head_target);
        //     // }

        //     render_element.redraw(&mut self.glyph_cx, pos, canvas);
        // } else {
        //     // We previously drew to the target layer, dirty it
        //     // if render_element.head.is_some() {
        //     //     dirty_layers.extend(render_element.head_target);
        //     // }

        //     render_element.clear();
        // }
    }

    pub fn render(&mut self) {
        let width = self.surface.config.width;
        let height = self.surface.config.height;
        let device_handle = &self.render_cx.devices[self.surface.dev_id];

        let surface_texture = self
            .surface
            .surface
            .get_current_texture()
            .expect("failed to get surface texture");

        let render_params = vello::RenderParams {
            base_color: Color::BLACK,
            width,
            height,
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            block_on_wgpu(
                &device_handle.device,
                self.renderer.render_to_surface_async(
                    &device_handle.device,
                    &device_handle.queue,
                    &self.scene,
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
                &self.scene,
                &surface_texture,
                &render_params,
            )
            .expect("failed to render to surface");

        surface_texture.present();

        device_handle.device.poll(wgpu::Maintain::Poll);
    }
}
