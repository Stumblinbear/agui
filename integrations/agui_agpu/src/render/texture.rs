use agpu::{BindGroup, Buffer, Texture};
use agui::unit::{BlendMode, Size};

pub struct RenderTextureId {
    pub size: Size,

    pub anti_alias: bool,
    pub blend_mode: BlendMode,
}

pub struct RenderTexture {
    pub texture: Texture<agpu::D2>,

    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}
