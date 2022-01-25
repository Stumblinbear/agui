use std::{
    cell::RefCell,
    fs::File,
    io::{self, BufReader, Read},
    mem,
};

use agpu::{
    wgpu::{self, TextureSampleType, TextureViewDimension},
    Frame, GpuHandle, RenderPipeline, Texture, TextureFormat,
};
use agui::{
    canvas::{command::CanvasCommand, font::FontId, paint::Brush, texture::TextureId, Canvas},
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

use crate::render::layer::LayerDrawTypes;

use self::{
    context::RenderContext,
    layer::{canvas::CanvasLayer, Layer, VertexData},
};

const INITIAL_FONT_CACHE_SIZE: (u32, u32) = (1024, 1024);

pub struct RenderEngine {
    layer_pipeline: RenderPipeline,

    ctx: RenderContext,

    layers: Vec<Layer>,
}

impl RenderEngine {
    pub fn new(gpu: &GpuHandle, size: Size) -> Self {
        const VERTEX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<VertexData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Vertex,
            attributes: &agpu::wgpu::vertex_attr_array![0 => Uint32],
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

        const BRUSHES_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
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

        const TYPE_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
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

        let layer_pipeline = gpu
            .new_pipeline("agui layer pipeline")
            .with_vertex(include_bytes!("shaders/layer.vert.spv"))
            .with_fragment(include_bytes!("shaders/layer.frag.spv"))
            .with_vertex_layouts(&[VERTEX_LAYOUT])
            .with_bind_groups(&[
                &gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        VIEWPORT_BINDING,
                        TYPE_BINDING,
                        BRUSHES_BINDING,
                        INDICES_BINDING,
                        POSITIONS_BINDING,
                        TEXTURE_BINDING,
                        SAMPLER_BINDING,
                    ],
                }),
            ])
            .create();

        Self {
            layer_pipeline,

            ctx: RenderContext {
                gpu: GpuHandle::clone(gpu),

                render_size: gpu
                    .new_buffer("agui render size")
                    .as_uniform_buffer()
                    .allow_copy_to()
                    .create(&[size.width, size.height]),

                draw_types: LayerDrawTypes::new(gpu),

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

            layers: Vec::default(),
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
        if let Some(root_id) = tree.get_root() {
            self.redraw_node(tree, root_id);
        } else {
            self.layers.clear();
        }
    }

    pub fn redraw_node<'ui>(&mut self, tree: &Tree<WidgetId, WidgetNode<'ui>>, node_id: WidgetId) {
        let mut layers: Vec<CanvasLayer> = Vec::default();
        let mut layer_stack: Vec<(usize, usize)> = Vec::default();

        tree.iter_from(node_id)
            .map(|widget_id| {
                tree.get_node(widget_id)
                    .expect("tree node missing during redraw")
            })
            .for_each(|node| {
                if !layer_stack.is_empty() {
                    // Pop any layer off the stack that has a higher depth than our current node
                    while !layer_stack.is_empty() {
                        let (depth, _idx) = layer_stack.last().unwrap();

                        // If the layer on the top of the stack has a higher (or equal) depth, then
                        // we've returned to a node higher in the tree than the layer we were building
                        if *depth >= node.depth {
                            layer_stack.pop();
                        } else {
                            break;
                        }
                    }
                } else {
                    // Even if the node doesn't draw anything, it should still begin a layer
                    // this generally only runs on the first node that is checked
                    layers.push(CanvasLayer::default());

                    layer_stack.push((node.depth, layers.len() - 1));
                }

                let painter = match node.painter.as_ref() {
                    Some(painter) => painter,
                    None => return,
                };

                let rect = match node.rect {
                    Some(rect) => rect,
                    None => return,
                };

                let mut canvas = Canvas::new(rect);

                painter.draw(&mut canvas);

                let commands = canvas.get_commands().clone();

                // If the canvas added no new commands, bail
                if commands.is_empty() {
                    return;
                }

                for cmd in commands {
                    match cmd {
                        CanvasCommand::Clip { rect, clip, shape } => {
                            layers.push(CanvasLayer {
                                clip: Some((rect, clip, shape)),

                                ..CanvasLayer::default()
                            });

                            layer_stack.push((node.depth, layers.len() - 1));
                        }

                        mut cmd => {
                            if let Some(brush) = cmd.get_brush() {
                                let layer = layers.last_mut().unwrap();

                                let paint = canvas.get_paint(brush);

                                if let Some(new_brush) = layer.paint_map.get(paint) {
                                    cmd.set_brush(*new_brush);
                                } else {
                                    let new_brush = Brush::from(layer.paint_map.len());

                                    layer.paint_map.insert(paint.clone(), new_brush);

                                    cmd.set_brush(new_brush);
                                }
                            }

                            layers.last_mut().unwrap().commands.push(cmd);
                        }
                    }
                }
            });

        self.layers.clear();

        for layer in layers {
            if let Some(layer) = layer.resolve(&mut self.ctx) {
                self.layers.push(layer);
            }
        }
    }

    pub fn render(&mut self, mut frame: Frame) {
        frame
            .render_pass_cleared("agui clear pass", 0x11111111)
            .begin();

        let mut r = frame
            .render_pass("agui layer pass")
            .with_pipeline(&self.layer_pipeline)
            .begin();

        for layer in &self.layers {
            for draw in &layer.draws {
                r.set_bind_group(0, &draw.bind_group, &[]);

                r.set_vertex_buffer(0, draw.vertex_data.slice(..))
                    .draw(0..draw.count, 0..1);
            }
        }
    }
}
