use wgpu::{BindGroup, Buffer, RenderPass};

pub struct DrawCall {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

impl DrawCall {
    pub fn render<'r>(&'r self, r: &mut RenderPass<'r>) {
        r.set_bind_group(0, &self.bind_group, &[]);

        r.set_vertex_buffer(1, self.vertex_data.slice(..));

        r.draw(0..self.count, 0..1);
    }
}
