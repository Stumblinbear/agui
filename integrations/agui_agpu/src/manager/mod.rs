use std::{cell::RefCell, mem, time::Instant};

use agpu::{
    wgpu::{self, TextureSampleType, TextureViewDimension},
    Frame, Gpu, RenderPipeline, TextureFormat,
};
use agui::{
    manager::{events::WidgetEvent, WidgetManager},
    unit::Size,
    widget::WidgetId,
};
use fnv::{FnvHashMap, FnvHashSet};
use glyph_brush_draw_cache::DrawCache;

mod element;

use crate::{
    context::PaintContext,
    manager::element::RenderElement,
    render::data::{InstanceData, VertexData},
};

const INITIAL_FONT_CACHE_SIZE: (u32, u32) = (1024, 1024);

pub(crate) struct RenderManager {
    pipeline: RenderPipeline,

    ctx: PaintContext,

    widgets: FnvHashMap<WidgetId, RenderElement>,
}

impl RenderManager {
    pub fn new(gpu: &Gpu, size: Size) -> Self {
        const INSTANCE_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Instance,
            attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x2],
        };

        const VERTEX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<VertexData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Vertex,
            attributes: &agpu::wgpu::vertex_attr_array![1 => Float32x4],
        };

        const VIEWPORT_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const OPTIONS_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const INDICES_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const POSITIONS_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const TEXTURE_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };

        const SAMPLER_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        };

        let pipeline = gpu
            .new_pipeline("agui layer pipeline")
            .with_vertex(include_bytes!("../shaders/layer.vert.spv"))
            .with_fragment(include_bytes!("../shaders/layer.frag.spv"))
            .with_vertex_layouts(&[INSTANCE_LAYOUT, VERTEX_LAYOUT])
            .with_bind_groups(&[
                &gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        VIEWPORT_BINDING,
                        OPTIONS_BINDING,
                        INDICES_BINDING,
                        POSITIONS_BINDING,
                        TEXTURE_BINDING,
                        SAMPLER_BINDING,
                    ],
                }),
            ])
            .create();

        Self {
            pipeline,

            ctx: PaintContext {
                gpu: Gpu::clone(gpu),

                render_size: gpu
                    .new_buffer("agui render size")
                    .as_uniform_buffer()
                    .allow_copy_to()
                    .create(&[size.width, size.height]),

                unknown_texture: gpu
                    .new_texture("agui unknown texture")
                    .allow_binding()
                    .create((1, 1), &[255_u8, 255, 255, 255]),
                texture_sampler: gpu.new_sampler("agui texture sampler").create(),

                textures: Vec::default(),

                font_texture: gpu
                    .new_texture("agui font texture")
                    .with_format(TextureFormat::R8Unorm)
                    .allow_binding()
                    .create_empty(INITIAL_FONT_CACHE_SIZE),

                font_draw_cache: RefCell::new(
                    DrawCache::builder()
                        .dimensions(INITIAL_FONT_CACHE_SIZE.0, INITIAL_FONT_CACHE_SIZE.1)
                        .build(),
                ),
            },

            // canvas_cache: HashMap::default(),
            // draw_cache: HashMap::default(),
            widgets: FnvHashMap::default(),
        }
    }

    pub fn set_size(&mut self, size: Size) {
        self.ctx
            .render_size
            .write_unchecked(&[size.width, size.height]);
    }

    pub fn redraw(&mut self, manager: &WidgetManager, events: &[WidgetEvent]) {
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
                    self.update_element(manager, *widget_id, &mut dirty_layers);
                }

                _ => todo!(),
            }
        }

        for layer in dirty_layers {
            self.redraw_layer(manager, layer);
        }

        tracing::info!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    fn update_element(
        &mut self,
        manager: &WidgetManager,
        widget_id: WidgetId,
        dirty_layers: &mut FnvHashSet<WidgetId>,
    ) {
        let element = self
            .widgets
            .get_mut(&widget_id)
            .expect("drawn render element not found");

        println!("updating element: {:?}", widget_id);

        let canvas = manager.get_widgets().get(widget_id).unwrap().paint();

        if let Some(canvas) = canvas {
            // If we have or are drawing to the target layer, mark it dirty
            if !canvas.head.is_empty() || element.head.is_some() {
                dirty_layers.extend(element.head_target);
            }

            let fonts = manager.get_fonts();

            element.update(&mut self.ctx, fonts, canvas);
        } else {
            // We previously drew to the target layer, dirty it
            if element.head.is_some() {
                dirty_layers.extend(element.head_target);
            }

            element.clear();
        }
    }

    fn redraw_layer(&mut self, manager: &WidgetManager, widget_id: WidgetId) {
        let element = self
            .widgets
            .get_mut(&widget_id)
            .expect("drawn render element not found");
    }

    pub fn render(&self, manager: &WidgetManager, mut frame: Frame) {
        frame
            .render_pass_cleared("agui clear pass", 0x11111111)
            .begin();

        let mut r = frame
            .render_pass("agui layer pass")
            .with_pipeline(&self.pipeline)
            .begin();

        for widget_id in manager.get_widgets().iter_down() {
            let element = self.widgets.get(&widget_id).unwrap();

            element.render(&mut r);
        }
    }
}
