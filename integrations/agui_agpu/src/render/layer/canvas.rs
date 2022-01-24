use std::collections::HashMap;

use agui::{
    canvas::{
        clipping::Clip,
        command::CanvasCommand,
        paint::{Brush, Paint},
        texture::TextureId,
    },
    unit::{Rect, Shape},
};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use crate::render::RenderContext;

use super::{BrushData, Layer, LayerShapes, ShapeVertexData};

#[derive(Debug, Default)]
pub struct CanvasLayer {
    pub clip: Option<(Rect, Clip, Shape)>,

    pub paint_map: HashMap<Paint, Brush>,

    pub commands: Vec<CanvasCommand>,

    pub textured: HashMap<TextureId, Vec<TextureId>>,
}

impl CanvasLayer {
    pub fn resolve(self, ctx: &RenderContext) -> Option<Layer> {
        let mut brush_data = vec![BrushData { color: [0.0; 4] }; self.paint_map.len()];

        for (paint, brush) in self.paint_map {
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

        for cmd in self.commands {
            match cmd {
                CanvasCommand::Shape { rect, brush, shape } => {
                    let count = tessellator
                        .tessellate_path(&shape.build_path(rect), &fill_options, &mut builder)
                        .unwrap();

                    vertex_data.resize(
                        vertex_data.len() + count.indices as usize,
                        ShapeVertexData {
                            brush_id: brush.idx() as u32,
                        },
                    );
                }

                CanvasCommand::Texture {
                    rect,
                    brush,
                    shape,
                    texture,
                    tex_bounds,
                } => {
                    
                }

                cmd => panic!("unknown command: {:?}", cmd),
            }
        }

        let mut layer = Layer {
            shapes: None,
            textured: Vec::default(),
        };

        if !vertex_data.is_empty() {
            layer.shapes = Some(LayerShapes {
                count: geometry.indices.len() as u32,

                vertex_data: ctx
                    .gpu
                    .new_buffer("agui layer vertex buffer")
                    .as_vertex_buffer()
                    .create(&vertex_data),

                bind_group: ctx.gpu.create_bind_group(&[
                    ctx.render_size.bind_uniform().in_vertex(),
                    ctx.gpu
                        .new_buffer("agui layer brush buffer")
                        .as_storage_buffer()
                        .create(&brush_data)
                        .bind_storage_readonly()
                        .in_vertex(),
                    ctx.gpu
                        .new_buffer("agui layer index buffer")
                        .as_storage_buffer()
                        .create(&geometry.indices)
                        .bind_storage_readonly()
                        .in_vertex(),
                    ctx.gpu
                        .new_buffer("agui layer position buffer")
                        .as_storage_buffer()
                        .create(&geometry.vertices)
                        .bind_storage_readonly()
                        .in_vertex(),
                ]),
            });
        }

        // No point in making a 0 size buffer
        if layer.shapes.is_none() && layer.textured.is_empty() {
            None
        } else {
            Some(layer)
        }
    }
}
