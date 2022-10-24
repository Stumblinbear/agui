#[repr(u32)]
#[derive(bytemuck::Contiguous, Debug, Clone, Copy)]
pub enum LayerDrawType {
    Texture = 0,
    Font = 1,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
pub struct LayerDrawOptions {
    pub r#type: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct InstanceData {
    pub pos: f32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct VertexData {
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct PositionData {
    pub xy: [f32; 2],
    pub uv: [f32; 2],
}
