use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    mem,
};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{
    unit::{Color, Rect},
    widget::{Widget, WidgetId},
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
}

pub struct QuadRenderPass {
    bind_group: BindGroup,

    pipeline: RenderPipeline,

    bound_widgets: HashSet<TypeId>,

    widgets: HashMap<WidgetId, Buffer>,
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
                array_stride: mem::size_of::<ShapeData>() as u64,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32, 2 => Float32x4],
            }])
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,

            bound_widgets: HashSet::new(),

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
    ) {
        if !self.bound_widgets.contains(type_id) {
            return;
        }

        let buffer = ctx
            .gpu
            .new_buffer("agui_shape_buffer")
            .as_vertex_buffer()
            .create(bytemuck::bytes_of(&ShapeData {
                rect: rect.to_slice(),
                z: 0.0,
                color: manager
                    .try_get_as::<Quad>(widget_id)
                    .and_then(|quad| quad.style.as_ref().map(|style| style.color))
                    .unwrap_or(Color::White)
                    .as_rgba(),
            }));

        self.widgets.insert(*widget_id, buffer);
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

        self.widgets.remove(widget_id);
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui_quad_pass")
            .with_pipeline(&self.pipeline)
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        for buffer in self.widgets.values() {
            r.set_vertex_buffer(0, buffer.slice(..)).draw(0..6, 0..1);
        }
    }
}
