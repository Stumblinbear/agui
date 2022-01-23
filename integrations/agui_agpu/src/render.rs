use std::mem;

use agpu::{Buffer, Frame, GpuHandle, GpuProgram, RenderPipeline};
use agui::{
    canvas::{clipping::Clip, command::CanvasCommand, Canvas},
    engine::node::WidgetNode,
    tree::Tree,
    unit::{Color, Rect, Shape},
    widget::WidgetId,
};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

pub struct RenderEngine {
    gpu: GpuHandle,
    pipeline: RenderPipeline,

    layers: Vec<Layer>,
}

impl RenderEngine {
    pub fn new(gpu: &GpuHandle) -> Self {
        let pipeline = gpu
            .new_pipeline("agui layer pipeline")
            .with_vertex(include_bytes!("shader/layer.vert.spv"))
            .with_fragment(include_bytes!("shader/layer.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<[f32; 2]>() as u64,
                step_mode: agpu::wgpu::VertexStepMode::Vertex,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
            }])
            .create();

        Self {
            gpu: GpuHandle::clone(gpu),
            pipeline,

            layers: Vec::default(),
        }
    }

    pub fn redraw<'ui>(&mut self, tree: &Tree<WidgetId, WidgetNode<'ui>>) {
        let mut layers: Vec<CanvasLayer> = Vec::default();
        let mut layer_stack: Vec<(usize, usize)> = Vec::default();

        tree.iter()
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

                let commands = canvas.get_commands();

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

                        cmd => {
                            // if cmd.is_noop() {
                            //     continue;
                            // }

                            layers.last_mut().unwrap().commands.push(cmd.clone());
                        }
                    }
                }
            });

        self.layers.clear();

        println!("{} layers:", layers.len());

        for layer in layers {
            println!("  - clip: {:?}", layer.clip);
            
            for cmd in layer.commands {
                println!("    | {:?}", cmd);
            }
        }

        // let mut geometry: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();

        // let mut builder = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
        //     vertex.position().to_array()
        // });

        // let mut tessellator = FillTessellator::new();

        // for cmd in commands {
        //     match cmd {
        //         CanvasCommand::Clip { rect, clip, shape } => todo!(),
        //         CanvasCommand::Shape { rect, brush, shape } => {
        //             let count = tessellator
        //                 .tessellate_path(
        //                     &shape.build_path(rect),
        //                     &FillOptions::default(),
        //                     &mut builder,
        //                 )
        //                 .unwrap();

        //             // vertices.push(vertex);
        //         }
        //         cmd => panic!("unknown command: {:?}", cmd),
        //     }
        // }

        // let layer_data = self
        //     .gpu
        //     .new_buffer("agui layer instance buffer")
        //     .as_vertex_buffer()
        //     .create(bytemuck::bytes_of(&LayerData {}));

        // let vertex_data = self
        //     .gpu
        //     .new_buffer("agui layer vertex buffer")
        //     .as_vertex_buffer()
        //     .create(&geometry.vertices);

        // let index_data = self
        //     .gpu
        //     .new_buffer("agui layer index buffer")
        //     .as_index_buffer()
        //     .create(&geometry.indices);

        // self.layers.push(Layer {
        //     layer_data,
        //     vertex_data,
        //     index_data,
        //     count: todo!(),
        // })
    }

    pub fn render(&mut self, frame: Frame) {
        // let mut r = frame
        //     .render_pass("agui layer pass")
        //     .with_pipeline(&self.pipeline)
        //     .begin();

        // for widget_buffer in self.widgets.values() {
        //     r.set_vertex_buffer(0, widget_buffer.drawable_data.slice(..))
        //         .set_vertex_buffer(1, widget_buffer.vertex_data.slice(..))
        //         .set_index_buffer(widget_buffer.index_data.slice(..))
        //         .draw_indexed(0..widget_buffer.count, 0, 0..1);
        // }
    }
}

#[derive(Debug, Default)]
struct CanvasLayer {
    clip: Option<(Rect, Clip, Shape)>,

    commands: Vec<CanvasCommand>,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct LayerData {}

pub struct Layer {
    commands: Vec<CanvasCommand>,

    layer_data: Buffer,

    vertex_data: Buffer,
    index_data: Buffer,
    count: u32,
}
