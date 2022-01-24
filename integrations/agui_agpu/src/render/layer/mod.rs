use agpu::{BindGroup, Buffer};

pub mod canvas;
pub mod shape;
pub mod textured;

pub struct Layer {
    pub shapes: Option<LayerShapes>,
    pub textured: Vec<LayerTextureShapes>,
}

pub struct LayerShapes {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

pub struct LayerTextureShapes {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct ShapeVertexData {
    pub brush_id: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
pub struct TexturedShapeVertexData {
    pub brush_id: u32,
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Debug, Copy, Clone)]
pub struct BrushData {
    pub color: [f32; 4],
}
