use agpu::{BindGroup, Buffer, GpuHandle};

pub mod canvas;

const TEXTURE_TYPE: u32 = 0;
const FONT_TYPE: u32 = 1;

pub struct LayerDrawTypes {
    pub texture: Buffer,
    pub font: Buffer,
}

impl LayerDrawTypes {
    pub fn new(gpu: &GpuHandle) -> Self {
        Self {
            texture: gpu
                .new_buffer("agui layer (texture)")
                .as_uniform_buffer()
                .create(&[TEXTURE_TYPE]),
            font: gpu
                .new_buffer("agui layer (font)")
                .as_uniform_buffer()
                .create(&[FONT_TYPE]),
        }
    }
}

pub struct Layer {
    pub draws: Vec<LayerDraw>,
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
