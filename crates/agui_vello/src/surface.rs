use std::time::Instant;

use agui_core::{
    element::ElementId,
    manager::{events::ElementEvent, WidgetManager},
    render::RenderContextId,
    unit::Offset,
};
use fnv::FnvHashMap;
use vello::{
    block_on_wgpu,
    kurbo::{Affine, Vec2},
    peniko::Color,
    util::{RenderContext, RenderSurface},
    Scene, SceneBuilder,
};

use crate::{element::RenderElement, fonts::VelloFonts};

pub struct VelloSurface {
    pub render_context_id: RenderContextId,

    pub surface: RenderSurface,
    pub renderer: vello::Renderer,

    pub scene: Scene,
    pub widgets: FnvHashMap<ElementId, RenderElement>,
}

impl VelloSurface {
    pub fn init(&mut self, widget_manager: &WidgetManager, fonts: &mut VelloFonts<'_>) {
        let boundary_element_id = widget_manager
            .get_render_context_manager()
            .get_boundary(self.render_context_id)
            .expect("the required render context boundary does not exist");

        let render_context_manager = widget_manager.get_render_context_manager();
        let tree = widget_manager.get_tree();

        let redraw_render_context_widgets = tree
            .iter_subtree(boundary_element_id, |element_id| {
                render_context_manager.get_context(element_id) == Some(self.render_context_id)
            })
            .flat_map(|element_id| {
                [
                    ElementEvent::Spawned {
                        parent_id: widget_manager.get_tree().get_parent(element_id).copied(),
                        element_id,
                    },
                    ElementEvent::Draw {
                        render_context_id: self.render_context_id,
                        element_id,
                    },
                ]
            })
            .collect::<Vec<_>>();

        self.redraw(widget_manager, fonts, &redraw_render_context_widgets);
    }

    pub fn redraw(
        &mut self,
        widget_manager: &WidgetManager,
        fonts: &mut VelloFonts<'_>,
        events: &[ElementEvent],
    ) {
        let now = Instant::now();

        for event in events {
            match event {
                ElementEvent::Spawned {
                    parent_id,
                    element_id,
                } => {
                    self.create_element(widget_manager, *element_id, *parent_id);
                }

                ElementEvent::Destroyed { element_id } => {
                    self.widgets.remove(element_id);
                }

                ElementEvent::Reparent {
                    parent_id,
                    element_id,
                } => {
                    // We need to check if a subtree was moved outside or into this render context
                    let was_in_render_context = self.widgets.contains_key(element_id);

                    let is_in_render_context = widget_manager
                        .get_render_context_manager()
                        .get_context(*element_id)
                        == Some(self.render_context_id);

                    if was_in_render_context && !is_in_render_context {
                        // Remove the subtree from the render context
                        for element_id in widget_manager
                            .get_tree()
                            .iter_subtree(*element_id, |element_id| {
                                self.widgets.contains_key(&element_id)
                            })
                            .collect::<Vec<_>>()
                        {
                            self.widgets.remove(&element_id);
                        }
                    } else if !was_in_render_context && is_in_render_context {
                        let render_context_id = self.render_context_id;

                        // Add the subtree to the render context
                        for element_id in
                            widget_manager
                                .get_tree()
                                .iter_subtree(*element_id, |element_id| {
                                    widget_manager
                                        .get_render_context_manager()
                                        .get_context(element_id)
                                        == Some(render_context_id)
                                })
                        {
                            self.create_element(widget_manager, element_id, *parent_id);
                        }
                    }

                    if is_in_render_context {
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

                ElementEvent::Draw {
                    render_context_id,
                    element_id,
                } => {
                    if *render_context_id != self.render_context_id {
                        continue;
                    }

                    self.update_element(widget_manager, fonts, *element_id);
                }

                ElementEvent::Rebuilt { .. } => {}

                _ => todo!(),
            }
        }

        let mut builder = SceneBuilder::for_scene(&mut self.scene);

        let mut element_stack = Vec::<(usize, ElementId, Affine)>::new();

        let boundary_element_id = widget_manager
            .get_render_context_manager()
            .get_boundary(self.render_context_id)
            .expect("the required render context boundary does not exist");

        let render_context_manager = widget_manager.get_render_context_manager();
        let tree = widget_manager.get_tree();

        for element_id in tree.iter_subtree(boundary_element_id, |element_id| {
            render_context_manager.get_context(element_id) == Some(self.render_context_id)
        }) {
            let element = self.widgets.get(&element_id).unwrap();

            let element_depth = widget_manager.get_tree().get_depth(element_id).unwrap();

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
        widget_manager: &WidgetManager,
        element_id: ElementId,
        parent_id: Option<ElementId>,
    ) {
        self.widgets.insert(
            element_id,
            RenderElement {
                head_target: parent_id.and_then(|parent_id| {
                    let Some(parent) = self.widgets.get(&parent_id) else {
                            if widget_manager.get_render_context_manager().get_context(parent_id) == Some(self.render_context_id) {
                                panic!("render element spawned to a non-existent parent");
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

                ..RenderElement::default()
            },
        );
    }

    fn update_element(
        &mut self,
        manager: &WidgetManager,
        fonts: &mut VelloFonts<'_>,
        element_id: ElementId,
    ) {
        let render_element = self
            .widgets
            .get_mut(&element_id)
            .expect("drawn render element not found");

        let widget_element = manager.get_tree().get(element_id).unwrap();

        let canvas = widget_element.paint();

        render_element.offset = widget_element.get_offset();

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

    pub fn render(&mut self, render_context: &RenderContext) {
        let width = self.surface.config.width;
        let height = self.surface.config.height;
        let device_handle = &render_context.devices[self.surface.dev_id];

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
