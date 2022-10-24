use agui::unit::Rect;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    Buffer,
};

use crate::context::RenderContext;

use super::texture::RenderTexture;

pub(crate) struct RenderLayer {
    pub rect: Rect,

    pub pos_buffer: Buffer,

    pub texture: RenderTexture,
}

impl RenderLayer {
    fn new(ctx: &mut RenderContext, rect: Rect) -> Self {
        Self {
            rect,

            pos_buffer: ctx.handle.device.create_buffer_init(&BufferInitDescriptor {
                label: Some("agui layer position buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&[rect.x, rect.y]),
            }),

            texture: RenderTexture::new(ctx, rect.into()),
        }
    }
}
