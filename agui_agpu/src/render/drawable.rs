use std::{any::TypeId, collections::HashMap, mem};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{
    unit::{Color, Rect},
    widget::WidgetId,
    widgets::primitives::Drawable,
    WidgetManager,
};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use super::{RenderContext, WidgetRenderPass};

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct DrawableData {
    layer: u32,
    color: [f32; 4],
}

struct WidgetBuffer {
    drawable_data: Buffer,

    vertex_data: Buffer,
    index_data: Buffer,
    count: u32,
}

pub struct DrawableRenderPass {
    last_size: (u32, u32),

    bind_group: BindGroup,
    pipeline: RenderPipeline,

    widgets: HashMap<WidgetId, WidgetBuffer>,
}

impl DrawableRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let bindings = &[
            ctx.bind_app_settings(),
            ctx.layer_mask.bind_storage_texture(),
            ctx.layer_mask_sampler.bind(),
        ];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui_drawable_pipeline")
            .with_vertex(include_bytes!("shader/quad.vert.spv"))
            .with_fragment(include_bytes!("shader/quad.frag.spv"))
            .with_vertex_layouts(&[
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<DrawableData>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Instance,
                    attributes: &agpu::wgpu::vertex_attr_array![0 => Uint32, 1 => Float32x4],
                },
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<[f32; 2]>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Vertex,
                    attributes: &agpu::wgpu::vertex_attr_array![2 => Float32x2],
                },
            ])
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            last_size: ctx.size,

            bind_group,
            pipeline,

            widgets: HashMap::default(),
        }
    }
}

impl WidgetRenderPass for DrawableRenderPass {
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
        layer: u32,
    ) {
        if type_id != &TypeId::of::<Drawable>() {
            return;
        }

        let drawable = manager.get_as::<Drawable>(widget_id);

        let geometry: VertexBuffers<[f32; 2], u16> = {
            let rect = manager
                .get_rect(widget_id)
                .expect("widget does not have a rect");

            let mut geometry = VertexBuffers::new();

            let mut tessellator = FillTessellator::new();
            {
                // Compute the tessellation.
                tessellator
                    .tessellate_path(
                        &drawable.shape.build_path(rect),
                        &FillOptions::default(),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                            vertex.position().to_array()
                        }),
                    )
                    .unwrap();
            }

            geometry
        };

        let drawable_data = ctx
            .gpu
            .new_buffer("agui_instance_buffer")
            .as_vertex_buffer()
            .create(bytemuck::bytes_of(&DrawableData {
                layer,
                color: drawable
                    .style
                    .as_ref()
                    .map_or(Color::default(), |style| style.color)
                    .as_rgba(),
            }));

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
            WidgetBuffer {
                drawable_data,

                vertex_data,
                index_data,
                count: geometry.indices.len() as u32,
            },
        );
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if type_id != &TypeId::of::<Drawable>() {
            return;
        }

        self.widgets.remove(widget_id);
    }

    fn update(&mut self, ctx: &RenderContext) {
        if ctx.size != self.last_size {
            self.last_size = ctx.size;

            let bindings = &[
                ctx.bind_app_settings(),
                ctx.layer_mask.bind_storage_texture(),
                ctx.layer_mask_sampler.bind(),
            ];

            self.bind_group = ctx.gpu.create_bind_group(bindings);
        }
    }

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui_drawable_pass")
            .with_pipeline(&self.pipeline)
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        for widget_buffer in self.widgets.values() {
            r.set_vertex_buffer(0, widget_buffer.drawable_data.slice(..))
                .set_vertex_buffer(1, widget_buffer.vertex_data.slice(..))
                .set_index_buffer(widget_buffer.index_data.slice(..))
                .draw_indexed(0..widget_buffer.count, 0, 0..1);
        }
    }
}
