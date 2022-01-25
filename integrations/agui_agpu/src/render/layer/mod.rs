use agpu::{BindGroup, Buffer};

pub mod canvas;

pub struct Layer {
    pub draws: Vec<LayerDraw>,
    pub font: Option<LayerDraw>,
}

pub struct LayerDraw {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Copy, Clone)]
pub struct BrushData {
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct VertexData {
    pub brush_id: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct PositionData {
    pub xy: [f32; 2],
    pub uv: [f32; 2],
}
