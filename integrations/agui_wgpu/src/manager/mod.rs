use std::{cell::RefCell, time::Instant};

use agui::{
    manager::{events::WidgetEvent, WidgetManager},
    unit::{Point, Size},
    widget::WidgetId,
};
use fnv::{FnvHashMap, FnvHashSet};
use glyph_brush_draw_cache::DrawCache;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    CommandEncoderDescriptor, Extent3d, Operations, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, SamplerDescriptor, TextureDescriptor, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};

mod element;
pub mod paint_pipeline;

use crate::{
    context::RenderContext, handle::RenderHandle, manager::element::RenderElement,
    pipelines::screen::ScreenPipeline, render::texture::RenderTexture, storage::RenderStorage,
};

const INITIAL_FONT_CACHE_SIZE: (u32, u32) = (1024, 1024);

pub(crate) struct RenderManager {
    screen_pipeline: ScreenPipeline,
    paint_pipeline: RenderPipeline,

    storage: RenderStorage,

    resize_to: Option<Size>,

    root_texture: Option<RenderTexture>,

    widgets: FnvHashMap<WidgetId, RenderElement>,
}

impl RenderManager {
    pub fn new(handle: &RenderHandle, size: Size) -> Self {
        let unknown_texture = handle.device.create_texture_with_data(
            &handle.queue,
            &TextureDescriptor {
                label: Some("agui unknown texture"),
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            },
            &[255_u8, 255, 255, 255],
        );

        let font_texture = handle.device.create_texture(&TextureDescriptor {
            label: Some("agui font texture"),
            size: Extent3d {
                width: INITIAL_FONT_CACHE_SIZE.0,
                height: INITIAL_FONT_CACHE_SIZE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        });

        let font_texture_view = font_texture.create_view(&TextureViewDescriptor::default());

        let storage = RenderStorage {
            render_size: handle.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("agui size buffer"),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[size.width, size.height]),
            }),

            unknown_texture_view: unknown_texture.create_view(&TextureViewDescriptor::default()),
            texture_sampler: handle.device.create_sampler(&SamplerDescriptor {
                label: Some("agui texture sampler"),
                ..Default::default()
            }),

            textures: Vec::default(),

            font_texture,
            font_texture_size: Size {
                width: INITIAL_FONT_CACHE_SIZE.0 as f32,
                height: INITIAL_FONT_CACHE_SIZE.1 as f32,
            },
            font_texture_view,

            font_draw_cache: RefCell::new(
                DrawCache::builder()
                    .dimensions(INITIAL_FONT_CACHE_SIZE.0, INITIAL_FONT_CACHE_SIZE.1)
                    .build(),
            ),
        };

        Self {
            screen_pipeline: ScreenPipeline::new(&handle.device),
            paint_pipeline: paint_pipeline::create(&handle.device),

            storage,

            resize_to: Some(size),

            root_texture: None,

            // canvas_cache: HashMap::default(),
            // draw_cache: HashMap::default(),
            widgets: FnvHashMap::default(),
        }
    }

    pub fn resize(&mut self, handle: &RenderHandle, size: Size) {
        handle.queue.write_buffer(
            &self.storage.render_size,
            0,
            bytemuck::cast_slice(&[size.width, size.height]),
        );

        self.resize_to = Some(size);
    }

    pub fn redraw(
        &mut self,
        handle: &RenderHandle,
        manager: &WidgetManager,
        events: &[WidgetEvent],
    ) {
        let now = Instant::now();

        let mut dirty_layers = FnvHashSet::default();

        for event in events {
            match event {
                WidgetEvent::Spawned {
                    parent_id,
                    widget_id,
                } => {
                    self.widgets.insert(
                        *widget_id,
                        RenderElement {
                            head_target: parent_id.and_then(|parent_id| {
                                let parent = self
                                    .widgets
                                    .get(&parent_id)
                                    .expect("render element spawned to a non-existent parent");

                                if parent.tail.is_some() {
                                    Some(parent_id)
                                } else {
                                    parent.head_target
                                }
                            }),

                            ..RenderElement::default()
                        },
                    );
                }

                WidgetEvent::Rebuilt { .. } => {}

                WidgetEvent::Reparent {
                    parent_id,
                    widget_id,
                } => {
                    let new_head_target = parent_id.and_then(|parent_id| {
                        let parent = self
                            .widgets
                            .get(&parent_id)
                            .expect("render element spawned to a non-existent parent");

                        if parent.tail.is_some() {
                            Some(parent_id)
                        } else {
                            parent.head_target
                        }
                    });

                    let element = self
                        .widgets
                        .get_mut(widget_id)
                        .expect("reparented render element not found");

                    dirty_layers.extend(element.head_target);
                    dirty_layers.extend(new_head_target);

                    element.head_target = new_head_target;
                }

                WidgetEvent::Destroyed { widget_id } => {
                    if let Some(element) = self.widgets.remove(widget_id) {
                        dirty_layers.extend(element.head_target);
                    }
                }

                WidgetEvent::Draw { widget_id } => {
                    self.update_element(handle, manager, *widget_id, &mut dirty_layers);
                }

                _ => todo!(),
            }
        }

        for layer in dirty_layers {
            self.redraw_layer(manager, layer);
        }

        let mut encoder = handle
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("agui redraw encoder"),
            });

        if let Some(resize_to) = self.resize_to.take() {
            self.root_texture = Some(RenderTexture::new(
                &RenderContext {
                    handle,

                    storage: &mut self.storage,
                },
                resize_to,
            ));
        }

        if let Some(root_texture) = &self.root_texture {
            let mut r = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("agui redraw pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &root_texture.view,
                    resolve_target: None,
                    ops: Operations {
                        // load: LoadOp::Load,
                        // store: true,
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            r.set_pipeline(&self.paint_pipeline);

            for widget_id in manager.get_widgets().iter_down() {
                let element = self.widgets.get(&widget_id).unwrap();

                element.render(&mut r);
            }
        }

        handle.queue.submit(Some(encoder.finish()));

        tracing::info!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    fn update_element(
        &mut self,
        handle: &RenderHandle,
        manager: &WidgetManager,
        widget_id: WidgetId,
        dirty_layers: &mut FnvHashSet<WidgetId>,
    ) {
        let render_element = self
            .widgets
            .get_mut(&widget_id)
            .expect("drawn render element not found");

        let widget_element = manager.get_widgets().get(widget_id).unwrap();

        let canvas = widget_element.paint();

        if let Some(canvas) = canvas {
            let pos = Point::from(widget_element.get_rect().unwrap());

            // If we have or are drawing to the target layer, mark it dirty
            if !canvas.head.is_empty() || render_element.head.is_some() {
                dirty_layers.extend(render_element.head_target);
            }

            let fonts = manager.get_fonts();

            render_element.update(
                &mut RenderContext {
                    handle,

                    storage: &mut self.storage,
                },
                fonts,
                pos,
                canvas,
            );
        } else {
            // We previously drew to the target layer, dirty it
            if render_element.head.is_some() {
                dirty_layers.extend(render_element.head_target);
            }

            render_element.clear();
        }
    }

    fn redraw_layer(&mut self, manager: &WidgetManager, widget_id: WidgetId) {
        let element = self
            .widgets
            .get_mut(&widget_id)
            .expect("drawn render element not found");
    }

    pub fn render(&mut self, handle: &RenderHandle, texture_view: &TextureView) {
        if let Some(root_texture) = &self.root_texture {
            let mut encoder = handle
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("agui render encoder"),
                });

            {
                let mut r = encoder.begin_render_pass(&RenderPassDescriptor {
                    label: Some("agui render pass"),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: texture_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.3,
                                g: 0.3,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                r.set_pipeline(&self.screen_pipeline.pipeline);

                root_texture.render(&mut r);
            }

            handle.queue.submit(Some(encoder.finish()));
        }
    }
}
