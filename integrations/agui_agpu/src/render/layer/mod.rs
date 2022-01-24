pub mod canvas;
pub mod shape;
pub mod textured;

use self::{shape::LayerShapes, textured::LayerTextureShapes};

pub struct Layer {
    pub shapes: Option<LayerShapes>,
    pub textured: Vec<LayerTextureShapes>,
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
    pub pos: [f32; 2],
}
