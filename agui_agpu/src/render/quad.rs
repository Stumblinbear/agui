use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{
    unit::{Color, Rect},
    widget::{Widget, WidgetId},
    widgets::primitives::Quad,
    WidgetManager,
};
use generational_arena::{Arena, Index as GenerationalIndex};

use super::{RenderContext, WidgetRenderPass};

const RECT_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const Z_BUFFER_SIZE: u64 = std::mem::size_of::<f32>() as u64;
const COLOR_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;

const QUAD_BUFFER_SIZE: u64 = RECT_BUFFER_SIZE + Z_BUFFER_SIZE + COLOR_BUFFER_SIZE;

const PREALLOCATE: u64 = QUAD_BUFFER_SIZE * 16;

// Make room for extra quads when we reach the buffer size, so we have to resize less often
const EXPAND_ALLOCATE: u64 = QUAD_BUFFER_SIZE * 8;

pub struct QuadRenderPass {
    bind_group: BindGroup,
    pipeline: RenderPipeline,
    buffer: Buffer,

    bound_widgets: HashSet<TypeId>,

    locations: Arena<WidgetId>,
    widgets: HashMap<WidgetId, GenerationalIndex>,
}

impl QuadRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let bindings = &[ctx.bind_app_settings()];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui_quad_pipeline")
            .with_vertex(include_bytes!("shader/rect.vert.spv"))
            .with_fragment(include_bytes!("shader/rect.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: QUAD_BUFFER_SIZE,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32, 2 => Float32x4],
            }])
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,
            buffer: program
                .gpu
                .new_buffer("agui_quad_buffer")
                .as_vertex_buffer()
                .allow_copy()
                .create_uninit(PREALLOCATE),

            bound_widgets: HashSet::new(),

            locations: Arena::default(),
            widgets: HashMap::default(),
        }
    }

    pub fn bind<W>(&mut self) -> &Self
    where
        W: Widget + 'static,
    {
        self.bound_widgets.insert(TypeId::of::<W>());

        self
    }

    pub fn get_buffer_index(&self, widget_id: &WidgetId) -> Option<u64> {
        let index = match self.widgets.get(widget_id) {
            Some(widget) => widget,
            None => return None,
        };

        let index = index.into_raw_parts().0 as u64;

        Some(index * QUAD_BUFFER_SIZE)
    }
}

impl WidgetRenderPass for QuadRenderPass {
    fn added(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        // Ignore any widget we aren't bound to
        if !self.bound_widgets.contains(type_id) {
            return;
        }

        let index = self.locations.insert(*widget_id);

        self.widgets.insert(*widget_id, index);
    }

    fn layout(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
        rect: &Rect,
    ) {
        if !self.bound_widgets.contains(type_id) {
            return;
        }

        let index = match self.get_buffer_index(widget_id) {
            Some(index) => index,
            None => return,
        };

        let rect = rect.to_slice();

        let rgba = manager
            .try_get_as::<Quad>(widget_id)
            .and_then(|quad| quad.style.as_ref().map(|style| style.color))
            .unwrap_or(Color::White)
            .as_rgba();

        let rect = bytemuck::cast_slice(&rect);
        let rgba = bytemuck::cast_slice(&rgba);

        if (self.buffer.size() as u64) < index + QUAD_BUFFER_SIZE {
            self.buffer
                .resize((self.buffer.size() as u64) + EXPAND_ALLOCATE);
        }

        ctx.gpu.queue.write_buffer(&self.buffer, index, rect);

        ctx.gpu.queue.write_buffer(
            &self.buffer,
            index + RECT_BUFFER_SIZE,
            bytemuck::cast_slice(&[0.0]),
        );

        ctx.gpu
            .queue
            .write_buffer(&self.buffer, index + RECT_BUFFER_SIZE + Z_BUFFER_SIZE, rgba);
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if !self.bound_widgets.contains(type_id) {
            return;
        }

        let index = self
            .widgets
            .remove(widget_id)
            .expect("removed nonexistent widget");

        self.locations.remove(index);

        // TODO: clear garbage from the buffer
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui_quad_pass")
            .with_pipeline(&self.pipeline)
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        r.set_vertex_buffer(0, self.buffer.slice(..))
            .draw(0..6, 0..(self.locations.capacity() as u32));
    }
}
