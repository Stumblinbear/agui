use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    mem,
    rc::{Rc, Weak},
    time::Instant,
};

use agpu::{
    wgpu::{self, TextureSampleType, TextureViewDimension},
    Frame, GpuHandle, RenderPipeline, TextureFormat,
};
use agui::{
    canvas::{paint::Brush, Canvas},
    engine::Engine,
    unit::Size,
    widget::WidgetId,
};
use glyph_brush_draw_cache::DrawCache;

mod context;
mod layer;

use crate::render::{context::RenderContext, layer::InstanceData};

use self::layer::{canvas::CanvasBufferBuilder, CanvasBuffer, RenderNode, VertexData};

const INITIAL_FONT_CACHE_SIZE: (u32, u32) = (1024, 1024);

pub struct RenderEngine {
    pipeline: RenderPipeline,

    ctx: RenderContext,

    cache: HashMap<u64, Weak<CanvasBuffer>>,

    nodes: Vec<RenderNode>,
}

impl RenderEngine {
    pub fn new(gpu: &GpuHandle, size: Size) -> Self {
        const INSTANCE_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Instance,
            attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x2],
        };

        const VERTEX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<VertexData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Vertex,
            attributes: &agpu::wgpu::vertex_attr_array![1 => Uint32],
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

        const BRUSHES_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const INDICES_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const POSITIONS_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const TEXTURE_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };

        const SAMPLER_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 6,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        };

        let pipeline = gpu
            .new_pipeline("agui layer pipeline")
            .with_vertex(include_bytes!("shaders/layer.vert.spv"))
            .with_fragment(include_bytes!("shaders/layer.frag.spv"))
            .with_vertex_layouts(&[INSTANCE_LAYOUT, VERTEX_LAYOUT])
            .with_bind_groups(&[
                &gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        VIEWPORT_BINDING,
                        OPTIONS_BINDING,
                        BRUSHES_BINDING,
                        INDICES_BINDING,
                        POSITIONS_BINDING,
                        TEXTURE_BINDING,
                        SAMPLER_BINDING,
                    ],
                }),
            ])
            .create();

        let render_size = gpu
            .new_buffer("agui render size")
            .as_uniform_buffer()
            .allow_copy_to()
            .create(&[size.width, size.height]);

        Self {
            pipeline,

            ctx: RenderContext {
                gpu: GpuHandle::clone(gpu),

                render_size,

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

            cache: HashMap::default(),

            nodes: Vec::default(),
        }
    }

    pub fn get_context(&self) -> &RenderContext {
        &self.ctx
    }

    pub fn set_size(&mut self, size: Size) {
        self.ctx
            .render_size
            .write_unchecked(&[size.width, size.height]);
    }

    pub fn redraw<'ui>(&mut self, engine: &'ui Engine<'ui>) {
        let now = Instant::now();

        if let Some(root_id) = engine.get_tree().get_root() {
            self.redraw_node(engine, root_id);
        } else {
            self.nodes.clear();
        }

        println!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    pub fn redraw_node<'ui>(&mut self, engine: &'ui Engine<'ui>, node_id: WidgetId) {
        let mut nodes: Vec<RenderNode> = Vec::default();

        let fonts = engine.get_fonts();

        let tree = engine.get_tree();

        tree.iter_from(node_id)
            .map(|widget_id| {
                tree.get_node(widget_id)
                    .expect("tree node missing during redraw")
            })
            .for_each(|node| {
                let render_func = match node.renderer.as_ref() {
                    Some(render_func) => render_func,
                    None => return,
                };

                let rect = match node.rect {
                    Some(rect) => rect,
                    None => return,
                };

                let mut canvas = Canvas::new(rect.into());

                render_func.call(&mut canvas);

                // If the canvas added no commands, bail
                if canvas.is_empty() {
                    return;
                }

                let mut hasher = DefaultHasher::new();
                canvas.hash(&mut hasher);
                let hash = hasher.finish();

                let canvas_buffer = if let Some(canvas_buffer) = self.cache.get(&hash) {
                    if let Some(canvas_buffer) = canvas_buffer.upgrade() {
                        canvas_buffer
                    } else {
                        panic!("attempted to pull a dead buffer from the cache");
                    }
                } else {
                    let mut builder = CanvasBufferBuilder {
                        fonts,
                        
                        clip: None,
                        paint_map: HashMap::default(),
                        commands: Vec::default(),
                    };

                    while let Some(mut cmd) = canvas.consume() {
                        if let Some(brush) = cmd.get_brush() {
                            let paint = canvas.get_paint(brush);

                            if let Some(new_brush) = builder.paint_map.get(paint) {
                                cmd.set_brush(*new_brush);
                            } else {
                                let new_brush = Brush::from(builder.paint_map.len());

                                builder.paint_map.insert(paint.clone(), new_brush);

                                cmd.set_brush(new_brush);
                            }
                        }

                        builder.commands.push(cmd);
                    }

                    let canvas_buffer = Rc::new(builder.build(&mut self.ctx));

                    self.cache.insert(hash, Rc::downgrade(&canvas_buffer));

                    canvas_buffer
                };

                nodes.push(RenderNode {
                    pos: self
                        .ctx
                        .gpu
                        .new_buffer("agui layer instance buffer")
                        .as_vertex_buffer()
                        .create(&[rect.x, rect.y]),
                    canvas_buffer,
                });
            });

        self.nodes = nodes;

        // Remove any invalidated buffers from the cache
        self.cache
            .retain(|_, canvas_buffer| canvas_buffer.upgrade().is_some());
    }

    pub fn render(&mut self, mut frame: Frame) {
        frame
            .render_pass_cleared("agui clear pass", 0x11111111)
            .begin();

        let mut r = frame
            .render_pass("agui layer pass")
            .with_pipeline(&self.pipeline)
            .begin();

        for node in &self.nodes {
            r.set_vertex_buffer(0, node.pos.slice(..));

            for layer in &node.canvas_buffer.layers {
                r.set_bind_group(0, &layer.bind_group, &[]);

                r.set_vertex_buffer(1, layer.vertex_data.slice(..))
                    .draw(0..layer.count, 0..1);
            }
        }
    }
}
