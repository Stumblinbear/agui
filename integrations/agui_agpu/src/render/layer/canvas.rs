use std::collections::HashMap;

use agui::{
    canvas::{
        clipping::Clip,
        command::CanvasCommand,
        font::{HorizontalAlign, VerticalAlign},
        paint::{Brush, Paint},
    },
    unit::{Rect, Shape},
};

use glyph_brush_draw_cache::CachedBy;
use glyph_brush_layout::{
    BuiltInLineBreaker, FontId as GlyphFontId, GlyphPositioner, Layout as GlyphLayout,
    SectionGeometry, SectionText,
};
use lyon::lyon_tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
};

use crate::render::{layer::LayerDrawOptions, RenderContext};

use super::{BrushData, CanvasBuffer, Layer, LayerDrawType, PositionData, VertexData};

#[derive(Debug, Default)]
pub struct CanvasBufferBuilder {
    pub clip: Option<(Rect, Clip, Shape)>,

    pub paint_map: HashMap<Paint, Brush>,

    pub commands: Vec<CanvasCommand>,
}

impl CanvasBufferBuilder {
    pub fn build(self, ctx: &mut RenderContext) -> CanvasBuffer {
        let mut brush_data = vec![BrushData { color: [0.0; 4] }; self.paint_map.len()];

        for (paint, brush) in self.paint_map {
            brush_data[brush.idx()] = BrushData {
                color: paint.color.into(),
            };
        }

        let mut vertex_data = Vec::default();
        let mut geometry: VertexBuffers<PositionData, u32> = VertexBuffers::new();
        let mut builder = BuffersBuilder::new(&mut geometry, |vertex: FillVertex| PositionData {
            xy: vertex.position().to_array(),
            uv: [0.0, 0.0],
        });
        let mut tessellator = FillTessellator::new();

        let mut glyphs = Vec::default();

        for cmd in &self.commands {
            match cmd {
                CanvasCommand::Clip { rect, clip, shape } => {}

                CanvasCommand::Shape { rect, brush, shape } => {
                    let count = tessellator
                        .tessellate_path(
                            &shape.build_path(*rect),
                            &FillOptions::default(),
                            &mut builder,
                        )
                        .unwrap();

                    vertex_data.resize(
                        vertex_data.len() + count.indices as usize,
                        VertexData {
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
                } => {}

                CanvasCommand::Text {
                    mut rect,
                    brush,
                    font,
                    text,
                } => {
                    let font_id = match font.font_id.idx() {
                        Some(font_id) => font_id,
                        None => {
                            log::warn!("attempted to draw text using a null font");
                            continue;
                        }
                    };

                    let glyphs_layout = GlyphLayout::Wrap {
                        line_breaker: BuiltInLineBreaker::UnicodeLineBreaker,
                        h_align: match font.h_align {
                            HorizontalAlign::Left => glyph_brush_layout::HorizontalAlign::Left,
                            HorizontalAlign::Center => {
                                rect.x += rect.width / 2.0;

                                glyph_brush_layout::HorizontalAlign::Center
                            }
                            HorizontalAlign::Right => {
                                rect.x += rect.width;

                                glyph_brush_layout::HorizontalAlign::Right
                            }
                        },
                        v_align: match font.v_align {
                            VerticalAlign::Top => glyph_brush_layout::VerticalAlign::Top,
                            VerticalAlign::Center => {
                                rect.y += rect.height / 2.0;

                                glyph_brush_layout::VerticalAlign::Center
                            }
                            VerticalAlign::Bottom => {
                                rect.y += rect.height;

                                glyph_brush_layout::VerticalAlign::Bottom
                            }
                        },
                    };

                    glyphs.extend(
                        glyphs_layout
                            .calculate_glyphs(
                                ctx.get_fonts(),
                                &SectionGeometry {
                                    screen_position: (rect.x, rect.y),
                                    bounds: (rect.width, rect.height),
                                },
                                &[SectionText {
                                    text: &text,
                                    scale: font.size.into(),
                                    font_id: GlyphFontId(font_id),
                                }],
                            )
                            .into_iter()
                            .map(|v| (brush, v)),
                    );
                }

                cmd => panic!("unknown command: {:?}", cmd),
            }
        }

        let mut canvas_buffer = CanvasBuffer {
            layers: Vec::default(),
        };

        if !vertex_data.is_empty() {
            canvas_buffer.layers.push(Layer {
                count: vertex_data.len() as u32,

                vertex_data: ctx
                    .gpu
                    .new_buffer("agui layer vertex buffer")
                    .as_vertex_buffer()
                    .create(&vertex_data),

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
                    ctx.unknown_texture.bind_texture().in_fragment(),
                    ctx.texture_sampler.bind().in_fragment(),
                ]),
            });
        }

        if !glyphs.is_empty() {
            for (_, sg) in &glyphs {
                ctx.font_draw_cache
                    .get_mut()
                    .queue_glyph(sg.font_id.0, sg.glyph.clone());
            }

            let cached_by = loop {
                match ctx.font_draw_cache.borrow_mut().cache_queued(
                    ctx.get_fonts(),
                    |rect, tex_data| {
                        ctx.font_texture.write_block(
                            (rect.min[0], rect.min[1]),
                            (rect.width(), rect.height()),
                            tex_data,
                        );
                    },
                ) {
                    Ok(cached_by) => break cached_by,
                    Err(_) => {
                        let size = ctx.font_texture.size;

                        ctx.font_texture.resize((size.0 + 32, size.1 + 32));
                    }
                }
            };

            if let CachedBy::Reordering = cached_by {
                todo!();
            } else {
                let mut vertex_data = Vec::with_capacity(glyphs.len());
                let mut index_data = Vec::with_capacity(glyphs.len() * 6);
                let mut position_data = Vec::with_capacity(glyphs.len() * 4);

                for (brush, sg) in glyphs.into_iter() {
                    if let Some((tex_coords, px_coords)) = ctx
                        .font_draw_cache
                        .borrow()
                        .rect_for(sg.font_id.0, &sg.glyph)
                    {
                        vertex_data.resize(
                            vertex_data.len() + 6,
                            VertexData {
                                brush_id: brush.idx() as u32,
                            },
                        );

                        let index = position_data.len() as u32;

                        position_data.push(PositionData {
                            xy: [px_coords.min.x, px_coords.min.y],
                            uv: [tex_coords.min.x, tex_coords.min.y],
                        });

                        position_data.push(PositionData {
                            xy: [px_coords.max.x, px_coords.min.y],
                            uv: [tex_coords.max.x, tex_coords.min.y],
                        });

                        position_data.push(PositionData {
                            xy: [px_coords.max.x, px_coords.max.y],
                            uv: [tex_coords.max.x, tex_coords.max.y],
                        });

                        position_data.push(PositionData {
                            xy: [px_coords.min.x, px_coords.max.y],
                            uv: [tex_coords.min.x, tex_coords.max.y],
                        });

                        index_data.push(index);
                        index_data.push(index + 1);
                        index_data.push(index + 3);

                        index_data.push(index + 1);
                        index_data.push(index + 2);
                        index_data.push(index + 3);
                    }
                }

                canvas_buffer.layers.push(Layer {
                    count: vertex_data.len() as u32,

                    vertex_data: ctx
                        .gpu
                        .new_buffer("agui layer vertex buffer")
                        .as_vertex_buffer()
                        .create(&vertex_data),

                    bind_group: ctx.gpu.create_bind_group(&[
                        ctx.render_size.bind_uniform().in_vertex(),
                        ctx.gpu
                            .new_buffer("agui layer draw options")
                            .as_uniform_buffer()
                            .create(bytemuck::bytes_of(&LayerDrawOptions {
                                r#type: LayerDrawType::Font as u32,
                            }))
                            .bind_uniform()
                            .in_vertex_fragment(),
                        ctx.gpu
                            .new_buffer("agui layer brush buffer")
                            .as_storage_buffer()
                            .create(&brush_data)
                            .bind_storage_readonly()
                            .in_vertex(),
                        ctx.gpu
                            .new_buffer("agui layer index buffer")
                            .as_storage_buffer()
                            .create(&index_data)
                            .bind_storage_readonly()
                            .in_vertex(),
                        ctx.gpu
                            .new_buffer("agui layer position buffer")
                            .as_storage_buffer()
                            .create(&position_data)
                            .bind_storage_readonly()
                            .in_vertex(),
                        ctx.font_texture.bind_texture().in_fragment(),
                        ctx.texture_sampler.bind().in_fragment(),
                    ]),
                });
            }
        }

        canvas_buffer
    }
}
