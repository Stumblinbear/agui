use std::{any::TypeId, collections::HashMap, mem};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{
    unit::{Color, Rect},
    widget::WidgetId,
    widgets::primitives::Quad,
    WidgetManager,
};

use super::{RenderContext, WidgetRenderPass};

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct ShapeData {
    rect: [f32; 4],
    z: f32,
    color: [f32; 4],
    radius: f32,
}

struct WidgetBuffer {
    rect: [f32; 4],
    buffer: Buffer,
}

pub struct QuadRenderPass {
    bind_group: BindGroup,

    pipeline: RenderPipeline,

    widgets: HashMap<WidgetId, WidgetBuffer>,
}

impl QuadRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let bindings = &[ctx.bind_app_settings()];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui_quad_pipeline")
            .with_vertex(include_bytes!("shader/quad.vert.spv"))
            .with_fragment(include_bytes!("shader/quad.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<ShapeData>() as u64,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32, 2 => Float32x4, 3 => Float32],
            }])
            .with_depth()
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,

            widgets: HashMap::default(),
        }
    }
}

impl WidgetRenderPass for QuadRenderPass {
    fn added(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        _type_id: &TypeId,
        _widget_id: &WidgetId,
    ) {
    }

    fn layout(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
        rect: &Rect,
        z: f32,
    ) {
        if type_id != &TypeId::of::<Quad>() {
            return;
        }

        let quad = manager.get_as::<Quad>(widget_id);

        let rect = rect.to_slice();

        let buffer = ctx
            .gpu
            .new_buffer("agui_shape_buffer")
            .as_vertex_buffer()
            .create(bytemuck::bytes_of(&ShapeData {
                rect,
                z,
                color: quad
                    .style
                    .as_ref()
                    .map_or(Color::default(), |style| style.color)
                    .as_rgba(),
                radius: quad.radius,
            }));

        self.widgets
            .insert(*widget_id, WidgetBuffer { rect, buffer });
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if type_id != &TypeId::of::<Quad>() {
            return;
        }

        self.widgets.remove(widget_id);
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui_quad_pass")
            .with_pipeline(&self.pipeline)
            .with_depth(ctx.depth_buffer.attach_depth())
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        for widget_buffer in self.widgets.values() {
            r.set_scissor_rect(
                widget_buffer.rect[0].floor() as u32,
                widget_buffer.rect[1].floor() as u32,
                widget_buffer.rect[2].ceil() as u32,
                widget_buffer.rect[3].ceil() as u32,
            );

            r.set_vertex_buffer(0, widget_buffer.buffer.slice(..))
                .draw(0..6, 0..1);
        }
    }
}
