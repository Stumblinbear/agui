use std::mem;

use agpu::{Buffer, Frame, GpuHandle, GpuProgram, RenderPipeline};
use agui::{
    canvas::{command::CanvasCommand, Canvas},
    engine::node::WidgetNode,
    tree::Tree,
    unit::Color,
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
            .with_vertex_layouts(&[
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<LayerData>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Instance,
                    attributes: &agpu::wgpu::vertex_attr_array![0 => Uint32, 1 => Float32x4],
                },
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<[f32; 2]>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Vertex,
                    attributes: &agpu::wgpu::vertex_attr_array![2 => Float32x2],
                },
            ])
            .create();

        Self {
            gpu: GpuHandle::clone(&gpu),
            pipeline,

            layers: Vec::default(),
        }
    }

    pub fn redraw<'ui>(&mut self, tree: &Tree<WidgetId, WidgetNode<'ui>>) {
        let mut commands: Vec<CanvasCommand> = Vec::new();

        tree.iter()
            .map(|widget_id| {
                tree.get(widget_id)
                    .expect("tree node missing during redraw")
            })
            .filter(|node| node.painter.is_some())
            .filter(|node| node.rect.has_value() && node.rect.read().is_some())
            .for_each(|node| {
                let rect = node.rect.read().expect("should not panic");
                let painter = node.painter.as_ref().expect("should not panic");

                let mut canvas = Canvas::new(rect);

                painter.draw(&mut canvas);

                commands.extend(canvas.get_commands().clone());
            });

        let mut geometry: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();

        let mut builder = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
            vertex.position().to_array()
        });

        let mut tessellator = FillTessellator::new();

        for cmd in commands {
            match cmd {
                CanvasCommand::Clip { rect, clip, shape } => todo!(),
                CanvasCommand::Shape { rect, brush, shape } => {
                    let count = tessellator
                        .tessellate_path(
                            &shape.build_path(rect),
                            &FillOptions::default(),
                            &mut builder,
                        )
                        .unwrap();

                    // vertices.push(vertex);
                }
                cmd => panic!("unknown command: {:?}", cmd),
            }
        }

        let layer_data = self
            .gpu
            .new_buffer("agui layer instance buffer")
            .as_vertex_buffer()
            .create(bytemuck::bytes_of(&LayerData {}));

        let vertex_data = self
            .gpu
            .new_buffer("agui layer vertex buffer")
            .as_vertex_buffer()
            .create(&geometry.vertices);

        let index_data = self
            .gpu
            .new_buffer("agui layer index buffer")
            .as_index_buffer()
            .create(&geometry.indices);

        self.layers.clear();

        self.layers.push(Layer {
            layer_data,
            vertex_data,
            index_data,
            count: todo!(),
        })
    }

    pub fn render(&mut self, frame: Frame) {
        let mut r = frame
            .render_pass("agui layer pass")
            .with_pipeline(&self.pipeline)
            .begin();

        for widget_buffer in self.widgets.values() {
            r.set_vertex_buffer(0, widget_buffer.drawable_data.slice(..))
                .set_vertex_buffer(1, widget_buffer.vertex_data.slice(..))
                .set_index_buffer(widget_buffer.index_data.slice(..))
                .draw_indexed(0..widget_buffer.count, 0, 0..1);
        }
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct LayerData {}

pub struct Layer {
    layer_data: Buffer,

    vertex_data: Buffer,
    index_data: Buffer,
    count: u32,
}
