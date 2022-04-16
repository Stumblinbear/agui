use std::rc::Rc;

use agpu::{BindGroup, Buffer};

pub mod builder;
pub mod canvas;

pub struct RenderNode {
    pub pos: Buffer,

    pub canvas_buffer: Rc<CanvasBuffer>,
}

pub struct CanvasBuffer {
    pub layers: Vec<Layer>,
}

pub struct Layer {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

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
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Clone, Copy)]
pub struct BrushData {
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct InstanceData {
    pub pos: f32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct VertexData {
    pub brush_id: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct PositionData {
    pub xy: [f32; 2],
    pub uv: [f32; 2],
}
