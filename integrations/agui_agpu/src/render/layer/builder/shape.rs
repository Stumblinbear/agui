use agui::canvas::command::CanvasCommand;
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use crate::render::{
    context::RenderContext,
    layer::{BrushData, Layer, LayerDrawOptions, LayerDrawType, PositionData, VertexData},
};

use super::{LayerBuilder, LayerType};

pub struct ShapeLayerBuilder {
    vertex_data: Vec<VertexData>,

    geometry: VertexBuffers<PositionData, u32>,
    tessellator: FillTessellator,
}

impl Default for ShapeLayerBuilder {
    fn default() -> Self {
        Self {
            vertex_data: Vec::default(),

            geometry: VertexBuffers::new(),
            tessellator: FillTessellator::default(),
        }
    }
}

impl LayerBuilder<'_> for ShapeLayerBuilder {
    fn get_type(&self) -> LayerType {
        LayerType::Shape
    }

    fn can_process(&self, cmd: &CanvasCommand) -> bool {
        matches!(cmd, CanvasCommand::Shape { .. })
    }

    fn process(&mut self, cmd: CanvasCommand) {
        if let CanvasCommand::Shape { rect, brush, shape } = cmd {
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
                    brush_id: brush.idx() as u32,
                },
            );
        }
    }

    fn build(&self, ctx: &mut RenderContext, brush_data: &[BrushData]) -> Option<Layer> {
        if self.vertex_data.is_empty() {
            return None;
        }

        Some(Layer {
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
                    .new_buffer("agui layer brush buffer")
                    .as_storage_buffer()
                    .create(brush_data)
                    .bind_storage_readonly()
                    .in_vertex(),
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
                ctx.unknown_texture.bind_texture().in_fragment(),
                ctx.texture_sampler.bind().in_fragment(),
            ]),
        })
    }
}
