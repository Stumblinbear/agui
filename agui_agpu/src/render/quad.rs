use std::collections::HashMap;

use agpu::{Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{render::color::Color, widgets::primitives::Quad, WidgetID, WidgetManager};
use generational_arena::{Arena, Index as GenerationalIndex};

use super::{RenderContext, WidgetRenderPass};

pub struct QuadRenderPass {
    pipeline: RenderPipeline,
    buffer: Buffer,

    locations: Arena<WidgetID>,
    widgets: HashMap<WidgetID, GenerationalIndex>,
}

const RECT_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const COLOR_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const QUAD_BUFFER_SIZE: u64 = RECT_BUFFER_SIZE + COLOR_BUFFER_SIZE;

const PREALLOCATE: u64 = QUAD_BUFFER_SIZE * 16;

// Make room for extra quads when we reach the buffer size, so we have to resize less often
const EXPAND_ALLOCATE: u64 = QUAD_BUFFER_SIZE * 8;

impl QuadRenderPass {
    pub fn new(program: &GpuProgram) -> QuadRenderPass {
        let pipeline = program
            .gpu
            .new_pipeline("agui_pipeline")
            .with_vertex(include_bytes!("shader/rect.vert.spv"))
            .with_fragment(include_bytes!("shader/rect.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<[f32; 8]>() as u64,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
            }])
            .create();

        QuadRenderPass {
            pipeline,

            buffer: program
                .gpu
                .new_buffer("QuadRenderPass")
                .as_vertex_buffer()
                .allow_copy_to()
                .create_uninit(PREALLOCATE),

            locations: Arena::default(),
            widgets: HashMap::default(),
        }
    }
}

impl WidgetRenderPass for QuadRenderPass {
    fn add(&mut self, ctx: &RenderContext, manager: &WidgetManager, widget_id: WidgetID) {
        let index = self.locations.insert(widget_id);
        self.widgets.insert(widget_id, index);

        let index = index.into_raw_parts().0 as u64;

        let rect = manager
            .get_rect(&widget_id)
            .expect("widget added to render pass does not have a rect")
            .to_slice();

        let quad = manager.try_get_as::<Quad>(widget_id);

        let rgba = quad.map_or(Color::White.as_rgba(), |q| q.color.as_rgba());

        let rect = bytemuck::cast_slice(&rect);
        let rgba = bytemuck::cast_slice(&rgba);

        if (self.buffer.size() as u64) < index + QUAD_BUFFER_SIZE {
            self.buffer
                .resize((self.buffer.size() as u64) + EXPAND_ALLOCATE);
        }

        ctx.gpu.queue.write_buffer(&self.buffer, index, rect);
        ctx.gpu
            .queue
            .write_buffer(&self.buffer, index + RECT_BUFFER_SIZE, rgba);
    }

    fn refresh(&mut self, _ctx: &RenderContext, _manager: &WidgetManager) {}

    fn remove(&mut self, _ctx: &RenderContext, _manager: &WidgetManager, widget_id: WidgetID) {
        if let Some(index) = self.widgets.remove(&widget_id) {
            self.locations.remove(index);
        }
    }

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("basic render pass")
            .with_pipeline(&self.pipeline)
            .begin();

        for (index, _) in self.locations.iter() {
            let index = (index.into_raw_parts().0 as u64) * QUAD_BUFFER_SIZE;

            r.set_vertex_buffer(0, self.buffer.slice(index..(index + QUAD_BUFFER_SIZE)))
                .draw(0..6, 0..1);
        }
    }
}
