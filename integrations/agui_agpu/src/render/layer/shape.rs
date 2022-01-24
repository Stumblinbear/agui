use agpu::{BindGroup, Buffer};

pub struct LayerShapes {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}