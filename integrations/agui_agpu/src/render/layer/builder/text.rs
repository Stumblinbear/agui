use agui::canvas::{
    command::CanvasCommand,
    font::{HorizontalAlign, VerticalAlign},
    paint::Brush,
};
use glyph_brush_draw_cache::CachedBy;
use glyph_brush_layout::{
    BuiltInLineBreaker, FontId as GlyphFontId, GlyphPositioner, Layout as GlyphLayout,
    SectionGeometry, SectionGlyph, SectionText,
};

use crate::render::{
    context::RenderContext,
    layer::{BrushData, Layer, LayerDrawOptions, LayerDrawType, PositionData, VertexData},
};

use super::{LayerBuilder, LayerType};

#[derive(Default)]
pub struct TextLayerBuilder<'builder> {
    glyphs: Vec<(&'builder Brush, SectionGlyph)>,
}

impl<'builder> LayerBuilder<'builder> for TextLayerBuilder<'builder> {
    fn get_type(&self) -> LayerType {
        LayerType::Text
    }

    fn can_process(&self, cmd: &CanvasCommand) -> bool {
        matches!(cmd, CanvasCommand::Text { .. })
    }

    fn process(&mut self, ctx: &mut RenderContext, cmd: &'builder CanvasCommand) {
        if let CanvasCommand::Text {
            mut rect,
            brush,

            font,

            text,
        } = cmd
        {
            let font_id = match font.font_id.idx() {
                Some(font_id) => font_id,
                None => {
                    log::warn!("attempted to draw text using a null font");
                    return;
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

            self.glyphs.extend(
                glyphs_layout
                    .calculate_glyphs(
                        ctx.get_fonts(),
                        &SectionGeometry {
                            screen_position: (rect.x, rect.y),
                            bounds: (rect.width, rect.height),
                        },
                        &[SectionText {
                            text,
                            scale: font.size.into(),
                            font_id: GlyphFontId(font_id),
                        }],
                    )
                    .into_iter()
                    .map(|v| (brush, v)),
            );
        }
    }

    fn build(&self, ctx: &mut RenderContext, brush_data: &[BrushData]) -> Option<Layer> {
        if self.glyphs.is_empty() {
            return None;
        }

        for (_, sg) in &self.glyphs {
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
            let mut vertex_data = Vec::with_capacity(self.glyphs.len());
            let mut index_data = Vec::with_capacity(self.glyphs.len() * 6);
            let mut position_data = Vec::with_capacity(self.glyphs.len() * 4);

            for (brush, sg) in self.glyphs.iter() {
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

            Some(Layer {
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
                        .create(brush_data)
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
            })
        }
    }
}
