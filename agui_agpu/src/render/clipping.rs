use std::{any::TypeId, collections::HashMap, mem};

use agpu::{
    BindGroup, Buffer, Frame, GpuProgram, RenderAttachmentBuild, RenderPipeline, Texture,
    TextureDimensions, TextureFormat,
};
use agui::{widget::WidgetId, WidgetManager};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use super::{RenderContext, WidgetRenderPass};

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct ClippingData {
    layer: u32,
}

struct ClippingBuffer {
    clipping_data: Buffer,

    vertex_data: Buffer,
    index_data: Buffer,
    count: u32,
}

pub struct ClippingRenderPass {
    last_size: (u32, u32),

    render_texture: Texture<agpu::D2>,

    bind_group: BindGroup,
    pipeline: RenderPipeline,

    widgets: HashMap<WidgetId, ClippingBuffer>,
    widget_order: Vec<WidgetId>,
}

impl ClippingRenderPass {
    pub fn new(_program: &GpuProgram, ctx: &RenderContext) -> Self {
        let render_texture = ctx
            .gpu
            .new_texture("agui clipping render texture")
            .with_format(TextureFormat::R32Uint)
            .as_render_target()
            .allow_copy_to()
            .allow_copy_from()
            .create_empty(ctx.size);

        let bindings = &[
            ctx.bind_app_settings(),
            ctx.layer_mask.bind_storage_texture(),
        ];

        let bind_group = ctx.gpu.create_bind_group(bindings);

        let pipeline = ctx
            .gpu
            .new_pipeline("agui clipping pipeline")
            .with_vertex(include_bytes!("shader/clipping.vert.spv"))
            .with_fragment(include_bytes!("shader/clipping.frag.spv"))
            .with_fragment_targets(&[agpu::wgpu::ColorTargetState {
                format: agpu::wgpu::TextureFormat::R32Uint,
                blend: None,
                write_mask: agpu::wgpu::ColorWrites::RED,
            }])
            .with_vertex_layouts(&[
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<ClippingData>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Instance,
                    attributes: &agpu::wgpu::vertex_attr_array![0 => Uint32],
                },
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<[f32; 2]>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Vertex,
                    attributes: &agpu::wgpu::vertex_attr_array![1 => Float32x2],
                },
            ])
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            last_size: ctx.size,

            render_texture,

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
        _type_id: &TypeId,
        widget_id: &WidgetId,
        layer: u32,
    ) {
        if let Some(clipping) = manager.get_clipping(widget_id).try_get() {
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

            let clipping_data = ctx
                .gpu
                .new_buffer("agui_instance_buffer")
                .as_vertex_buffer()
                .create(bytemuck::bytes_of(&ClippingData { layer }));

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
        _type_id: &TypeId,
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

    fn update(&mut self, ctx: &RenderContext) {
        if ctx.size != self.last_size {
            self.last_size = ctx.size;
            
            self.render_texture.resize(ctx.size);

            let bindings = &[
                ctx.bind_app_settings(),
                ctx.layer_mask.bind_storage_texture(),
            ];

            self.bind_group = ctx.gpu.create_bind_group(bindings);
        }
    }

    fn render(&self, ctx: &RenderContext, _frame: &mut Frame) {
        let mut encoder = ctx
            .gpu
            .create_command_encoder("agui clipping command encoder");

        {
            encoder
                .render_pass(
                    "agui clipping clear",
                    &[self.render_texture.attach_render().clear_color(0)],
                )
                .begin();
        }

        for clipping_buffer in self.widgets.values() {
            {
                let mut r = encoder
                    .render_pass("agui clipping pass", &[self.render_texture.attach_render()])
                    .with_pipeline(&self.pipeline)
                    .begin();

                r.set_bind_group(0, &self.bind_group, &[]);

                r.set_vertex_buffer(0, clipping_buffer.clipping_data.slice(..))
                    .set_vertex_buffer(1, clipping_buffer.vertex_data.slice(..))
                    .set_index_buffer(clipping_buffer.index_data.slice(..))
                    .draw_indexed(0..clipping_buffer.count, 0, 0..1);
            }

            encoder.copy_texture_to_texture(
                self.render_texture.as_image_copy(),
                ctx.layer_mask.as_image_copy(),
                self.render_texture.size.as_extent(),
            );
        }
    }
}
