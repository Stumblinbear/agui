use agpu::Buffer;
use agui::unit::Rect;

use crate::context::RenderContext;

use super::texture::RenderTexture;

pub(crate) struct RenderLayer {
    pub rect: Rect,

    pub pos: Buffer,

    pub texture: RenderTexture,
}

impl RenderLayer {
    fn new(ctx: &mut RenderContext, rect: Rect) -> Self {
        Self {
            rect,

            pos: ctx
                .gpu
                .new_buffer("agui layer position buffer")
                .as_vertex_buffer()
                .create(&[rect.x, rect.y]),

            texture: RenderTexture::new(ctx, rect.into()),
        }
    }
}
