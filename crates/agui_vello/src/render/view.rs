use std::{sync::Arc, time::Instant};

use agui_core::{
    element::{Element, ElementId},
    engine::Engine,
    render::{object::RenderObject, RenderObjectId},
    unit::{Offset, Size},
    util::tree::Tree,
};
use parking_lot::Mutex;
use rustc_hash::FxHashMap;
use vello::{
    block_on_wgpu,
    kurbo::{Affine, Vec2},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    Scene, SceneBuilder,
};

use crate::render::{fonts::VelloFonts, VelloRenderObject};

pub struct VelloViewRenderer {
    pub fonts: Arc<Mutex<VelloFonts>>,

    pub render_context: RenderContext,

    pub surface: RenderSurface,
    pub renderer: vello::Renderer,

    pub scene: Scene,
    pub render_objects: FxHashMap<RenderObjectId, VelloRenderObject>,
}

impl VelloViewRenderer {
    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
        render_object: &RenderObject,
    ) {
        self.render_objects.insert(
            render_object_id,
            VelloRenderObject {
                head_target: parent_render_object_id.and_then(|parent_render_object_id| {
                    let Some(parent) = self.render_objects.get(&parent_render_object_id) else {
                        panic!(
                            "render object {:?} spawned to a non-existent parent {:?}",
                            render_object_id, parent_render_object_id
                        );
                    };

                    if parent.canvas.tail.is_some() {
                        Some(parent_render_object_id)
                    } else {
                        parent.head_target
                    }
                }),

                offset: Offset::ZERO,

                ..VelloRenderObject::default()
            },
        );
    }

    fn on_detach(&mut self, render_object_id: RenderObjectId) {
        self.render_objects.remove(&render_object_id);
    }

    pub fn redraw(
        &mut self,
        render_view_manager: &RenderViewManager,
        tree: &Tree<ElementId, Element>,
        fonts: &mut VelloFonts,
    ) {
        let now = Instant::now();

        let mut builder = SceneBuilder::for_scene(&mut self.scene);

        let mut element_stack = Vec::<(usize, ElementId, Affine)>::new();

        let boundary_element_id = render_view_manager
            .get_boundary(self.render_view_id)
            .expect("the required render view boundary does not exist");

        for element_id in tree.iter_subtree(boundary_element_id, |element_id| {
            render_view_manager.get_view(element_id) == Some(self.render_view_id)
        }) {
            let element = self.widgets.get(&element_id).unwrap();

            let element_depth = tree.get_depth(element_id).unwrap();

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

        tracing::debug!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    fn create_element(&mut self, tree: &Tree<ElementId, Element>, element_id: ElementId) {}

    fn update_element(&mut self, engine: &Engine, fonts: &mut VelloFonts, element_id: ElementId) {
        let render_element = self
            .widgets
            .get_mut(&element_id)
            .expect("drawn render element not found");

        let widget_element = engine.elements().get(element_id).unwrap();

        let canvas = widget_element.paint();

        render_element.offset = widget_element.offset();

        render_element.canvas.update(fonts, canvas);

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

    fn resize(&mut self, size: Size) {
        self.render_context.resize_surface(
            &mut self.surface,
            size.width as u32,
            size.height as u32,
        );
    }

    fn render(&mut self) {
        let width = self.surface.config.width;
        let height = self.surface.config.height;
        let device_handle = &self.render_context.devices[self.surface.dev_id];

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

        // device_handle.device.poll(wgpu::Maintain::Poll);
    }
}
