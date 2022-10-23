use agpu::{BindGroup, Buffer, Texture};
use agui::unit::{BlendMode, Size};

use crate::context::RenderContext;

pub struct RenderTexture {
    pub texture: Texture<agpu::D2>,

    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

impl RenderTexture {
    pub fn new(ctx: &mut RenderContext, size: Size) -> Self {
        Self {
            texture: ctx.gpu.new_texture("agui layer").allow_binding().create(
                (size.width.round() as u32, size.height.round() as u32),
                &[255_u8, 255, 255, 255],
            ),

            count: 0,
            vertex_data: ctx
                .gpu
                .new_buffer("agui render texture vertex data")
                .as_vertex_buffer()
                .create(&[0.0f32, 0.0f32]),
            bind_group: ctx
                .gpu
                .create_bind_group(&[ctx.render_size.bind_uniform().in_vertex()]),
        }
    }
}
