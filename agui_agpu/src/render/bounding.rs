use std::{
    any::TypeId,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{
    unit::{Color, Rect},
    widget::WidgetId,
    widgets::primitives::Text,
    WidgetManager,
};
use generational_arena::{Arena, Index as GenerationalIndex};

use super::{RenderContext, WidgetRenderPass};

pub struct BoundingRenderPass {
    bind_group: BindGroup,

    pipeline: RenderPipeline,
    buffer: Buffer,

    locations: Arena<WidgetId>,
    widgets: HashMap<WidgetId, GenerationalIndex>,
}

const RECT_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const Z_BUFFER_SIZE: u64 = std::mem::size_of::<f32>() as u64;
const COLOR_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;

const BOUNDING_BUFFER_SIZE: u64 = RECT_BUFFER_SIZE + Z_BUFFER_SIZE + COLOR_BUFFER_SIZE;

const PREALLOCATE: u64 = BOUNDING_BUFFER_SIZE * 16;

// Make room for extra quads when we reach the buffer size, so we have to resize less often
const EXPAND_ALLOCATE: u64 = BOUNDING_BUFFER_SIZE * 8;

impl BoundingRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let bindings = &[ctx.bind_app_settings()];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui_bounding_pipeline")
            .with_vertex(include_bytes!("shader/bounding.vert.spv"))
            .with_fragment(include_bytes!("shader/bounding.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: BOUNDING_BUFFER_SIZE,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32, 2 => Float32x4],
            }])
            .wireframe()
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,

            buffer: program
                .gpu
                .new_buffer("agui_bounding_buffer")
                .as_vertex_buffer()
                .allow_copy()
                .create_uninit(PREALLOCATE),

            locations: Arena::default(),
            widgets: HashMap::default(),
        }
    }
}

impl WidgetRenderPass for BoundingRenderPass {
    fn added(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if type_id != &TypeId::of::<Text>() {
            return;
        }

        let index = self.locations.insert(*widget_id);
        self.widgets.insert(*widget_id, index);
    }

    fn layout(
        &mut self,
        ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
        rect: &Rect,
    ) {
        let index = match self.widgets.get(widget_id) {
            Some(widget) => widget,
            None => return,
        };

        let index = index.into_raw_parts().0 as u64;

        let index = index * BOUNDING_BUFFER_SIZE;

        let rect = rect.to_slice();

        let rect = bytemuck::cast_slice(&rect);

        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        let c = hasher.finish().to_ne_bytes();
        let c = [
            (c[0] as f32) / 255.0,
            (c[1] as f32) / 255.0,
            (c[2] as f32) / 255.0,
            1.0,
        ];

        let rgba = bytemuck::cast_slice(&c);

        if (self.buffer.size() as u64) < index + BOUNDING_BUFFER_SIZE {
            self.buffer
                .resize((self.buffer.size() as u64) + EXPAND_ALLOCATE);
        }

        ctx.gpu.queue.write_buffer(&self.buffer, index, rect);
        
        ctx.gpu.queue.write_buffer(
            &self.buffer,
            index + RECT_BUFFER_SIZE,
            bytemuck::cast_slice(&[10.0]),
        );

        ctx.gpu
            .queue
            .write_buffer(&self.buffer, index + RECT_BUFFER_SIZE + Z_BUFFER_SIZE, rgba);
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        _type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if let Some(index) = self.widgets.remove(widget_id) {
            self.locations.remove(index);
        }
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui_bounding_pass")
            .with_pipeline(&self.pipeline)
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        r.set_vertex_buffer(0, self.buffer.slice(..))
            .draw(0..6, 0..(self.locations.capacity() as u32));
    }
}
