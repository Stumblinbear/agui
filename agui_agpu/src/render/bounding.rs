use std::{
    any::TypeId,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    mem,
};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{widget::WidgetId, WidgetManager};

use super::{RenderContext, WidgetRenderPass};

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct BoundingData {
    rect: [f32; 4],
    z: f32,
    color: [f32; 4],
}

pub struct BoundingRenderPass {
    bind_group: BindGroup,

    pipeline: RenderPipeline,

    widgets: HashMap<WidgetId, Buffer>,
}

impl BoundingRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let bindings = &[ctx.bind_app_settings()];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui bounding pipeline")
            .with_vertex(include_bytes!("shader/bounding.vert.spv"))
            .with_fragment(include_bytes!("shader/bounding.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: mem::size_of::<BoundingData>() as u64,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32, 2 => Float32x4],
            }])
            .wireframe()
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,

            widgets: HashMap::default(),
        }
    }
}

impl WidgetRenderPass for BoundingRenderPass {
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
        _depth: u32,
    ) {
        let rect = match manager.get_rect(widget_id) {
            Some(rect) => rect,
            None => return,
        };

        let mut hasher = DefaultHasher::new();
        type_id.hash(&mut hasher);
        let c = hasher.finish().to_ne_bytes();

        let rect = rect.to_slice();

        let buffer = ctx
            .gpu
            .new_buffer("agui bounding buffer")
            .as_vertex_buffer()
            .create(bytemuck::bytes_of(&BoundingData {
                rect: [
                    // Ensure the bounding box always shows on screen (not hidden on either 0 axis)
                    if rect[0] > -f32::EPSILON && rect[0] < f32::EPSILON {
                        1.0
                    } else {
                        rect[0]
                    },
                    if rect[1] > -f32::EPSILON && rect[1] < f32::EPSILON {
                        1.0
                    } else {
                        rect[1]
                    },
                    if rect[0] > -f32::EPSILON && rect[0] < f32::EPSILON {
                        rect[2] - 1.0
                    } else {
                        rect[2]
                    },
                    if rect[1] > -f32::EPSILON && rect[1] < f32::EPSILON {
                        rect[3] - 1.0
                    } else {
                        rect[3]
                    },
                ],
                z: 0.0,
                color: [
                    (c[0] as f32) / 255.0,
                    (c[1] as f32) / 255.0,
                    (c[2] as f32) / 255.0,
                    1.0,
                ],
            }));

        self.widgets.insert(*widget_id, buffer);
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        _type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        self.widgets.remove(widget_id);
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui bounding pass")
            .with_pipeline(&self.pipeline)
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        for buffer in self.widgets.values() {
            r.set_vertex_buffer(0, buffer.slice(..)).draw(0..6, 0..1);
        }
    }
}
