use std::{sync::Arc, time::Instant};

use agui_core::{
    element::{Element, ElementId},
    engine::Engine,
    unit::{Offset, Size},
    util::{map::ElementMap, tree::Tree},
};
use agui_renderer::{RenderViewId, RenderViewManager, ViewRenderer};
use parking_lot::Mutex;
use vello::{
    block_on_wgpu,
    kurbo::{Affine, Vec2},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    Scene, SceneBuilder,
};

use crate::fonts::VelloFonts;
use crate::render::RenderObject;

pub struct VelloViewRendererHandle {
    inner: Mutex<VelloViewRenderer>,
}

impl VelloViewRendererHandle {
    pub(crate) fn new(renderer: VelloViewRenderer) -> Self {
        Self {
            inner: Mutex::new(renderer),
        }
    }

    pub(crate) fn redraw(&self, engine: &Engine, fonts: &mut VelloFonts) {
        let render_view_manager =
            RenderViewManager::of(engine).expect("render view manager not found");

        let tree = engine.elements();

        self.inner.lock().redraw(render_view_manager, tree, fonts);
    }
}

impl ViewRenderer for VelloViewRendererHandle {
    fn resize(&self, size: Size) {
        self.inner.lock().resize(size);
    }

    fn render(&self) {
        self.inner.lock().render();
    }
}

pub struct VelloViewRenderer {
    pub fonts: Arc<Mutex<VelloFonts>>,

    pub render_context: RenderContext,

    pub render_view_id: RenderViewId,

    pub surface: RenderSurface,
    pub renderer: vello::Renderer,

    pub scene: Scene,
    pub widgets: ElementMap<RenderObject>,
}

impl VelloViewRenderer {
    pub fn init(&mut self, engine: &Engine, fonts: &mut VelloFonts) {
        let render_view_manager =
            RenderViewManager::of(engine).expect("render view manager not found");

        let boundary_element_id = render_view_manager
            .get_boundary(self.render_view_id)
            .expect("the required render view boundary does not exist");

        let tree = engine.elements();

        self.init_subtree(render_view_manager, tree, boundary_element_id);

        self.redraw(render_view_manager, tree, fonts);
    }

    pub fn init_subtree(
        &mut self,
        render_view_manager: &RenderViewManager,
        tree: &Tree<ElementId, Element>,
        boundary_element_id: ElementId,
    ) {
        for element_id in tree.iter_subtree(boundary_element_id, |element_id| {
            render_view_manager.get_view(element_id) == Some(self.render_view_id)
        }) {
            self.create_element(render_view_manager, tree, element_id);
        }
    }

    pub fn update_elements(&mut self, engine: &Engine, fonts: &mut VelloFonts) {
        let now = Instant::now();

        let tree = engine.elements();

        let render_view_manager =
            RenderViewManager::of(engine).expect("render view manager not found");

        for event in events {
            match event {
                ElementEvent::Spawned {
                    parent_id,
                    element_id,
                } => {
                    self.create_element(render_view_manager, *element_id, *parent_id);
                }

                ElementEvent::Destroyed { element_id } => {
                    self.widgets.remove(element_id);
                }

                ElementEvent::Reparent {
                    parent_id,
                    element_id,
                } => {
                    // We need to check if a subtree was moved outside or into this render view
                    let was_in_render_view = self.widgets.contains_key(element_id);

                    let is_in_render_view =
                        render_view_manager.get_view(*element_id) == Some(self.render_view_id);

                    if was_in_render_view && !is_in_render_view {
                        // Remove the subtree from the render view
                        for element_id in engine
                            .elements()
                            .iter_subtree(*element_id, |element_id| {
                                self.widgets.contains_key(&element_id)
                            })
                            .collect::<Vec<_>>()
                        {
                            self.widgets.remove(&element_id);
                        }
                    } else if !was_in_render_view && is_in_render_view {
                        let render_view_id = self.render_view_id;

                        self.init_subtree(render_view_manager, tree, *element_id);
                    }

                    if is_in_render_view {
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
                }

                _ => todo!(),
            }
        }

        tracing::info!(
            "updated render elements in: {:?}",
            Instant::now().duration_since(now)
        );

        self.redraw(render_view_manager, tree, fonts);
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

        tracing::info!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    fn create_element(
        &mut self,
        render_view_manager: &RenderViewManager,
        tree: &Tree<ElementId, Element>,
        element_id: ElementId,
    ) {
        let parent_id = tree.get_parent(element_id).copied();

        self.widgets.insert(
            element_id,
            RenderObject {
                head_target: parent_id.and_then(|parent_id| {
                    let Some(parent) = self.widgets.get(&parent_id) else {
                        // If the parent isn't tracked in the render view, but it's in the same context, then
                        // the something went wrong. The parent should always exist before the child is spawned.
                        if render_view_manager.get_view(parent_id)
                            == Some(self.render_view_id)
                        {
                            panic!(
                                "render element {:?} spawned to a non-existent parent {:?} in render view {:?}",
                                element_id, parent_id, self.render_view_id
                            );
                        }

                        return None;
                    };

                    if parent.canvas.tail.is_some() {
                        Some(parent_id)
                    } else {
                        parent.head_target
                    }
                }),

                offset: Offset::ZERO,

                ..RenderObject::default()
            },
        );
    }

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
