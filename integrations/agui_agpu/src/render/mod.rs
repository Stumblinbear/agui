use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap},
    fs::File,
    hash::{Hash, Hasher},
    io::{self, BufReader, Read},
    mem,
    rc::{Rc, Weak},
    time::Instant,
};

use agpu::{
    wgpu::{self, TextureSampleType, TextureViewDimension},
    Frame, GpuHandle, RenderPipeline, Texture, TextureFormat,
};
use agui::{
    canvas::{font::FontId, paint::Brush, texture::TextureId, Canvas},
    engine::node::WidgetNode,
    tree::Tree,
    unit::Size,
    widget::WidgetId,
};
use glyph_brush_draw_cache::{
    ab_glyph::{FontArc, InvalidFont},
    DrawCache,
};

mod context;
mod layer;

use crate::render::layer::InstanceData;

use self::{
    context::RenderContext,
    layer::{canvas::CanvasBufferBuilder, CanvasBuffer, RenderNode, VertexData},
};

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

                fonts: Vec::default(),

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

    pub fn set_size(&mut self, size: Size) {
        self.ctx
            .render_size
            .write_unchecked(&[size.width, size.height]);
    }

    pub fn load_texture(&mut self, texture: Texture<agpu::D2>) -> TextureId {
        self.ctx.load_texture(texture)
    }

    pub fn load_font_bytes(&mut self, bytes: &'static [u8]) -> Result<FontId, InvalidFont> {
        let font = FontArc::try_from_slice(bytes)?;

        Ok(self.ctx.load_font(font))
    }

    pub fn load_font_file(&mut self, filename: &str) -> io::Result<FontId> {
        let f = File::open(filename)?;

        let mut reader = BufReader::new(f);

        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes)?;

        let font = FontArc::try_from_vec(bytes)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        Ok(self.ctx.load_font(font))
    }

    pub fn redraw<'ui>(&mut self, tree: &Tree<WidgetId, WidgetNode<'ui>>) {
        let now = Instant::now();

        if let Some(root_id) = tree.get_root() {
            self.redraw_node(tree, root_id);
        } else {
            // self.nodes.clear();
        }

        println!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    pub fn redraw_node<'ui>(&mut self, tree: &Tree<WidgetId, WidgetNode<'ui>>, node_id: WidgetId) {
        let mut nodes: Vec<RenderNode> = Vec::default();

        tree.iter_from(node_id)
            .map(|widget_id| {
                tree.get_node(widget_id)
                    .expect("tree node missing during redraw")
            })
            .for_each(|node| {
                let painter = match node.painter.as_ref() {
                    Some(painter) => painter,
                    None => return,
                };

                let rect = match node.rect {
                    Some(rect) => rect,
                    None => return,
                };

                let mut canvas = Canvas::new(rect.into());

                painter.draw(&mut canvas);

                let commands = canvas.get_commands().clone();

                // If the canvas added no commands, bail
                if commands.is_empty() {
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
                        clip: None,
                        paint_map: HashMap::default(),
                        commands: Vec::default(),
                    };

                    for mut cmd in commands {
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
        let now = Instant::now();

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

        println!("rendered in: {:?}", Instant::now().duration_since(now));
    }
}
