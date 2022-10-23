use agpu::{BindGroup, Buffer, RenderPass};

pub struct DrawCall {
    pub count: u32,
    pub vertex_data: Buffer,
    pub bind_group: BindGroup,
}

impl DrawCall {
    pub fn render<'pass>(&'pass self, r: &mut RenderPass<'pass>) {
        r.set_bind_group(0, &self.bind_group, &[]);

        r.set_vertex_buffer(1, self.vertex_data.slice(..))
            .draw(0..self.count, 0..1);
    }
}
