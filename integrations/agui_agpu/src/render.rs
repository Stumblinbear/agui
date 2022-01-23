use std::{collections::HashMap, mem};

use agpu::{
    wgpu::{self},
    BindGroup, Buffer, Frame, GpuHandle, RenderPipeline,
};
use agui::{
    canvas::{
        clipping::Clip,
        command::CanvasCommand,
        paint::{Brush, Paint},
        Canvas,
    },
    engine::node::WidgetNode,
    tree::Tree,
    unit::{Rect, Shape, Size},
    widget::WidgetId,
};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

pub struct RenderEngine {
    gpu: GpuHandle,
    pipeline: RenderPipeline,

    render_size: Buffer,

    layers: Vec<Layer>,
}

impl RenderEngine {
    pub fn new(gpu: &GpuHandle, size: Size) -> Self {
        let binding = gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline = gpu
            .new_pipeline("agui layer pipeline")
            .with_vertex(include_bytes!("shader/layer.vert.spv"))
            .with_fragment(include_bytes!("shader/layer.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<VertexData>() as u64,
                step_mode: agpu::wgpu::VertexStepMode::Vertex,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Uint32],
            }])
            .with_bind_groups(&[&binding])
            .create();

        Self {
            gpu: GpuHandle::clone(gpu),
            pipeline,

            render_size: gpu
                .new_buffer("agui render size")
                .as_uniform_buffer()
                .allow_copy_to()
                .create(&[size.width, size.height]),

            layers: Vec::default(),
        }
    }

    pub fn set_size(&mut self, size: Size) {
        self.render_size.write_unchecked(&[size.width, size.height]);
    }

    pub fn clear(&mut self) {
        self.layers.clear();
    }

    pub fn redraw<'ui>(&mut self, tree: &Tree<WidgetId, WidgetNode<'ui>>, node_id: WidgetId) {
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
            let mut brush_data = vec![BrushData { color: [0.0; 4] }; layer.paint_map.len()];

            for (paint, brush) in layer.paint_map {
                brush_data[brush.idx()] = BrushData {
                    color: paint.color.into(),
                };
            }

            let mut vertex_data = Vec::default();

            let mut geometry: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();

            let mut builder = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                vertex.position().to_array()
            });

            let mut tessellator = FillTessellator::new();

            let fill_options = FillOptions::default();

            for cmd in layer.commands {
                match cmd {
                    CanvasCommand::Shape { rect, brush, shape } => {
                        let count = tessellator
                            .tessellate_path(&shape.build_path(rect), &fill_options, &mut builder)
                            .unwrap();

                        vertex_data.resize(
                            vertex_data.len() + count.indices as usize,
                            VertexData {
                                brush_id: brush.idx() as u32,
                            },
                        );
                    }

                    cmd => panic!("unknown command: {:?}", cmd),
                }
            }

            // No point in making a 0 size buffer
            if vertex_data.is_empty() {
                continue;
            }

            self.layers.push(Layer {
                count: geometry.indices.len() as u32,

                vertex_data: self
                    .gpu
                    .new_buffer("agui layer vertex buffer")
                    .as_vertex_buffer()
                    .create(&vertex_data),

                bind_group: self.gpu.create_bind_group(&[
                    self.render_size.bind_uniform().in_vertex(),
                    self.gpu
                        .new_buffer("agui layer brush buffer")
                        .as_storage_buffer()
                        .create(&brush_data)
                        .bind_storage_readonly()
                        .in_vertex(),
                    self.gpu
                        .new_buffer("agui layer index buffer")
                        .as_storage_buffer()
                        .create(&geometry.indices)
                        .bind_storage_readonly()
                        .in_vertex(),
                    self.gpu
                        .new_buffer("agui layer position buffer")
                        .as_storage_buffer()
                        .create(&geometry.vertices)
                        .bind_storage_readonly()
                        .in_vertex(),
                ]),
            })
        }
    }

    pub fn render(&mut self, mut frame: Frame) {
        let mut r = frame
            .render_pass_cleared("agui layer pass", 0x44444444)
            .with_pipeline(&self.pipeline)
            .begin();

        for layer in &self.layers {
            r.set_bind_group(0, &layer.bind_group, &[]);

            r.set_vertex_buffer(0, layer.vertex_data.slice(..))
                .draw(0..layer.count, 0..1);
        }
    }
}

#[derive(Debug, Default)]
struct CanvasLayer {
    clip: Option<(Rect, Clip, Shape)>,

    paint_map: HashMap<Paint, Brush>,

    commands: Vec<CanvasCommand>,
}

pub struct Layer {
    count: u32,

    vertex_data: Buffer,

    bind_group: BindGroup,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct VertexData {
    brush_id: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Copy, Clone)]
struct BrushData {
    color: [f32; 4],
}
