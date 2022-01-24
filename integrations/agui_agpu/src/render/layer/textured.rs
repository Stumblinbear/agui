use agpu::{BindGroup, Buffer};

pub struct LayerTextureShapes {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct TexCoordsData {
    pub uv: [f32; 2],
}
