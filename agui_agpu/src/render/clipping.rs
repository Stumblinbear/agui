use std::{any::TypeId, collections::HashMap, mem};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{widget::WidgetId, WidgetManager};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use super::{RenderContext, WidgetRenderPass};

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct ClippingData {
    z: f32,
}

struct ClippingBuffer {
    clipping_data: Buffer,

    vertex_data: Buffer,
    index_data: Buffer,
    count: u32,
}

pub struct ClippingRenderPass {
    bind_group: BindGroup,

    pipeline: RenderPipeline,

    widgets: HashMap<WidgetId, ClippingBuffer>,
    widget_order: Vec<WidgetId>,
}

impl ClippingRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let bindings = &[ctx.bind_app_settings()];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui_clipping_pipeline")
            .with_vertex(include_bytes!("shader/clipping.vert.spv"))
            .with_fragment(include_bytes!("shader/clipping.frag.spv"))
            .with_vertex_layouts(&[
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<ClippingData>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Instance,
                    attributes: &agpu::wgpu::vertex_attr_array![0 => Float32],
                },
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<[f32; 2]>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Vertex,
                    attributes: &agpu::wgpu::vertex_attr_array![1 => Float32x2],
                },
            ])
            .with_depth()
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,

            widgets: HashMap::default(),
            widget_order: Vec::default(),
        }
    }
}

impl WidgetRenderPass for ClippingRenderPass {
    fn added(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        _type_id: &TypeId,
        _widget_id: &WidgetId,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
        depth: f32,
    ) {
        if let Some(clipping) = manager.get_clipping(&widget_id).try_get() {
            let rect = manager
                .get_rect(widget_id)
                .expect("cannot have clipping on a widget with no size");

            let path = clipping.build_path(rect);

            let mut geometry: VertexBuffers<[f32; 2], u16> = VertexBuffers::new();

            let mut tessellator = FillTessellator::new();
            {
                // Compute the tessellation.
                tessellator
                    .tessellate_path(
                        &path,
                        &FillOptions::default(),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                            vertex.position().to_array()
                        }),
                    )
                    .unwrap();
            }

            println!("clip: {}", depth);

            let clipping_data = ctx
                .gpu
                .new_buffer("agui_instance_buffer")
                .as_vertex_buffer()
                .create(bytemuck::bytes_of(&ClippingData { z: depth }));

            let vertex_data = ctx
                .gpu
                .new_buffer("agui_vertex_buffer")
                .as_vertex_buffer()
                .create(&geometry.vertices);

            let index_data = ctx
                .gpu
                .new_buffer("agui_index_buffer")
                .as_index_buffer()
                .create(&geometry.indices);

            self.widgets.insert(
                *widget_id,
                ClippingBuffer {
                    clipping_data,

                    vertex_data,
                    index_data,
                    count: geometry.indices.len() as u32,
                },
            );

            self.widget_order.push(*widget_id);
        }
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if self.widgets.remove(widget_id).is_some() {
            self.widget_order.remove(
                self.widget_order
                    .iter()
                    .position(|id| id == widget_id)
                    .unwrap(),
            );
        }
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui clipping pass")
            .with_pipeline(&self.pipeline)
            .with_depth(ctx.depth_buffer.attach_depth())
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        for widget_buffer in self.widgets.values() {
            r.set_vertex_buffer(0, widget_buffer.clipping_data.slice(..))
                .set_vertex_buffer(1, widget_buffer.vertex_data.slice(..))
                .set_index_buffer(widget_buffer.index_data.slice(..))
                .draw_indexed(0..widget_buffer.count, 0, 0..1);
        }
    }
}
