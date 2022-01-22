use agpu::Buffer;

// #[repr(C)]
// #[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
#[derive(Default)]
pub struct Layer {
    children: Vec<Layer>,
    // drawable_data: Buffer,

    // vertex_data: Buffer,
    // index_data: Buffer,
    // count: u32,
}
