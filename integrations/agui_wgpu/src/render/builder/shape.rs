use agui::render::{canvas::command::CanvasCommand, texture::TextureId};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindingResource,
};

use crate::{
    context::RenderContext,
    manager::paint_pipeline::PAINT_PIPELINE_LAYOUT,
    render::{
        data::{LayerDrawOptions, LayerDrawType, PositionData, VertexData},
        draw_call::DrawCall,
    },
};

use super::DrawCallBuilder;

pub struct LayerShapeBuilder {
    texture_id: TextureId,

    vertex_data: Vec<VertexData>,

    geometry: VertexBuffers<PositionData, u32>,
    tessellator: FillTessellator,
}

impl LayerShapeBuilder {
    pub fn new(texture_id: TextureId) -> Self {
        Self {
            texture_id,

            vertex_data: Vec::default(),

            geometry: VertexBuffers::new(),
            tessellator: FillTessellator::default(),
        }
    }
}

impl DrawCallBuilder<'_> for LayerShapeBuilder {
    fn can_process(&self, cmd: &CanvasCommand) -> bool {
        match cmd {
            CanvasCommand::Shape { .. } => true,

            CanvasCommand::Texture { texture_id, .. } => self.texture_id == *texture_id,

            _ => false,
        }
    }

    fn process(&mut self, cmd: &CanvasCommand) {
        if let CanvasCommand::Shape { rect, shape, color } = cmd {
            let mut builder =
                BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| PositionData {
                    xy: vertex.position().to_array(),
                    uv: [0.0, 0.0],
                });

            let count = self
                .tessellator
                .tessellate_path(
                    &shape.build_path(*rect),
                    &FillOptions::default(),
                    &mut builder,
                )
                .unwrap();

            self.vertex_data.resize(
                self.vertex_data.len() + count.indices as usize,
                VertexData {
                    color: (*color).into(),
                },
            );
        }
    }

    fn build(&self, ctx: &mut RenderContext) -> Option<DrawCall> {
        if self.vertex_data.is_empty() {
            return None;
        }

        let draw_options_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::bytes_of(&LayerDrawOptions {
                r#type: LayerDrawType::Texture as u32,
            }),
        });

        let index_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&self.geometry.indices),
        });

        let position_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&self.geometry.vertices),
        });

        let bind_group_layout = ctx
            .handle
            .device
            .create_bind_group_layout(&PAINT_PIPELINE_LAYOUT);

        let bind_group = ctx.handle.device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: ctx.storage.render_size.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: draw_options_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: index_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: position_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(match self.texture_id.idx() {
                        Some(idx) => ctx.storage.textures.get(idx).unwrap(),
                        None => &ctx.storage.unknown_texture_view,
                    }),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&ctx.storage.texture_sampler),
                },
            ],
        });

        Some(DrawCall {
            count: self.vertex_data.len() as u32,

            vertex_data: ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&self.vertex_data),
            }),

            bind_group,
        })
    }
}
