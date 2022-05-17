use agui::canvas::{command::CanvasCommand, texture::TextureId};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use crate::{
    context::RenderContext,
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

    fn process(&mut self, cmd: CanvasCommand) {
        if let CanvasCommand::Shape { rect, shape, color } = cmd {
            let mut builder =
                BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| PositionData {
                    xy: vertex.position().to_array(),
                    uv: [0.0, 0.0],
                });

            let count = self
                .tessellator
                .tessellate_path(
                    &shape.build_path(rect),
                    &FillOptions::default(),
                    &mut builder,
                )
                .unwrap();

            self.vertex_data.resize(
                self.vertex_data.len() + count.indices as usize,
                VertexData {
                    color: color.into(),
                },
            );
        }
    }

    fn build(&self, ctx: &mut RenderContext) -> Option<DrawCall> {
        if self.vertex_data.is_empty() {
            return None;
        }

        Some(DrawCall {
            count: self.vertex_data.len() as u32,

            vertex_data: ctx
                .gpu
                .new_buffer("agui layer vertex buffer")
                .as_vertex_buffer()
                .create(&self.vertex_data),

            bind_group: ctx.gpu.create_bind_group(&[
                ctx.render_size.bind_uniform().in_vertex(),
                ctx.gpu
                    .new_buffer("agui layer draw options")
                    .as_uniform_buffer()
                    .create(bytemuck::bytes_of(&LayerDrawOptions {
                        r#type: LayerDrawType::Texture as u32,
                    }))
                    .bind_uniform()
                    .in_vertex_fragment(),
                ctx.gpu
                    .new_buffer("agui layer index buffer")
                    .as_storage_buffer()
                    .create(&self.geometry.indices)
                    .bind_storage_readonly()
                    .in_vertex(),
                ctx.gpu
                    .new_buffer("agui layer position buffer")
                    .as_storage_buffer()
                    .create(&self.geometry.vertices)
                    .bind_storage_readonly()
                    .in_vertex(),
                match self.texture_id.idx() {
                    Some(idx) => ctx.textures.get(idx).unwrap().bind_texture().in_fragment(),
                    None => ctx.unknown_texture.bind_texture().in_fragment(),
                },
                ctx.texture_sampler.bind().in_fragment(),
            ]),
        })
    }
}
