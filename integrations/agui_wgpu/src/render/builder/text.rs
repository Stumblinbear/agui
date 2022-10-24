use agui::render::canvas::command::CanvasCommand;
use glyph_brush_draw_cache::{ab_glyph::FontArc, CachedBy};
use glyph_brush_layout::SectionGlyph;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindingResource, Extent3d, Origin3d, TextureFormat,
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

#[derive(Default)]
pub struct TextDrawCallBuilder<'builder> {
    pub fonts: &'builder [FontArc],

    pub glyphs: Vec<([f32; 4], SectionGlyph)>,
}

impl<'builder> DrawCallBuilder<'builder> for TextDrawCallBuilder<'builder> {
    fn can_process(&self, cmd: &CanvasCommand) -> bool {
        matches!(cmd, CanvasCommand::Text { .. })
    }

    fn process(&mut self, cmd: &CanvasCommand) {
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
                font.get_glyphs(*rect, text)
                    .into_iter()
                    .map(|v| ((*color).into(), v)),
            );
        }
    }

    fn build(&self, ctx: &mut RenderContext) -> Option<DrawCall> {
        if self.glyphs.is_empty() {
            return None;
        }

        for (_, sg) in &self.glyphs {
            ctx.storage
                .font_draw_cache
                .get_mut()
                .queue_glyph(sg.font_id.0, sg.glyph.clone());
        }

        let cached_by = loop {
            match ctx.storage.font_draw_cache.borrow_mut().cache_queued(
                self.fonts,
                |rect, tex_data| {
                    ctx.handle.queue.write_texture(
                        wgpu::ImageCopyTextureBase {
                            texture: &ctx.storage.font_texture,
                            mip_level: 0,
                            origin: Origin3d {
                                x: rect.min[0],
                                y: rect.min[1],
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        tex_data,
                        wgpu::ImageDataLayout {
                            // This is 0 because our source should not be offset
                            offset: 0,
                            bytes_per_row: std::num::NonZeroU32::new(
                                rect.width() * TextureFormat::R8Unorm.describe().block_size as u32,
                            ),
                            rows_per_image: None,
                        },
                        Extent3d {
                            width: rect.width(),
                            height: rect.height(),
                            depth_or_array_layers: 1,
                        },
                    )
                },
            ) {
                Ok(cached_by) => break cached_by,
                Err(_) => {
                    let size = ctx.storage.font_texture_size;

                    // ctx.storage.font_texture_size = Size {
                    //     width: size.width + 32.0,
                    //     height: size.height + 32.0,
                    // };

                    // ctx.storage.font_texture.resize((size.0 + 32, size.1 + 32));
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
                    .storage
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

            let draw_options_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::bytes_of(&LayerDrawOptions {
                    r#type: LayerDrawType::Font as u32,
                }),
            });

            let index_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&index_data),
            });

            let position_buffer = ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&position_data),
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
                        resource: BindingResource::TextureView(&ctx.storage.font_texture_view),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(&ctx.storage.texture_sampler),
                    },
                ],
            });

            Some(DrawCall {
                count: vertex_data.len() as u32,

                vertex_data: ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                    label: None,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    contents: bytemuck::cast_slice(&vertex_data),
                }),

                bind_group,
            })
        }
    }
}
