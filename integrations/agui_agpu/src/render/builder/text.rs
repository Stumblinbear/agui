use agui::canvas::command::CanvasCommand;
use glyph_brush_draw_cache::{ab_glyph::FontArc, CachedBy};
use glyph_brush_layout::SectionGlyph;

use crate::{
    context::RenderContext,
    render::{
        data::{LayerDrawOptions, LayerDrawType, PositionData, VertexData},
        draw_call::DrawCall,
    },
};

use super::DrawCallBuilder;

#[derive(Default)]
pub struct TextDrawCallBuilder<'builder> {
    pub fonts: &'builder [FontArc],

    pub glyphs: Vec<([f32; 4], SectionGlyph)>,
}

impl<'builder> DrawCallBuilder<'builder> for TextDrawCallBuilder<'builder> {
    fn can_process(&self, cmd: &CanvasCommand) -> bool {
        matches!(cmd, CanvasCommand::Text { .. })
    }

    fn process(&mut self, cmd: CanvasCommand) {
        if let CanvasCommand::Text {
            rect,

            color,

            font,

            text,
        } = cmd
        {
            if font.get().is_none() {
                return;
            }

            self.glyphs.extend(
                font.get_glyphs(rect, &text)
                    .into_iter()
                    .map(|v| (color.into(), v)),
            );
        }
    }

    fn build(&self, ctx: &mut RenderContext) -> Option<DrawCall> {
        if self.glyphs.is_empty() {
            return None;
        }

        for (_, sg) in &self.glyphs {
            ctx.font_draw_cache
                .get_mut()
                .queue_glyph(sg.font_id.0, sg.glyph.clone());
        }

        let cached_by = loop {
            match ctx
                .font_draw_cache
                .borrow_mut()
                .cache_queued(self.fonts, |rect, tex_data| {
                    ctx.font_texture.write_block(
                        (rect.min[0], rect.min[1]),
                        (rect.width(), rect.height()),
                        tex_data,
                    );
                }) {
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

            for (color, sg) in self.glyphs.iter() {
                if let Some((tex_coords, px_coords)) = ctx
                    .font_draw_cache
                    .borrow()
                    .rect_for(sg.font_id.0, &sg.glyph)
                {
                    vertex_data.resize(vertex_data.len() + 6, VertexData { color: *color });

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

            Some(DrawCall {
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
