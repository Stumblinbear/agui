use std::{any::TypeId, collections::HashMap, mem};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline};
use agui::{
    unit::{ClippingMask, Color},
    widget::WidgetId,
    widgets::primitives::Quad,
    WidgetManager,
};
use lyon::{
    geom::euclid::{Point2D, Size2D},
    lyon_tessellation::{BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers},
    math::{Point, Rect},
    path::{builder::BorderRadii, traits::PathBuilder, Path, Winding},
};

use super::{RenderContext, WidgetRenderPass};

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct QuadData {
    z: f32,
    color: [f32; 4],
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct VertexData([f32; 2]);

struct WidgetBuffer {
    quad_data: Buffer,

    vertex_data: Buffer,
    index_data: Buffer,
    count: u32,
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
            .with_vertex_layouts(&[
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<QuadData>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Instance,
                    attributes: &agpu::wgpu::vertex_attr_array![0 => Float32, 1 => Float32x4],
                },
                agpu::wgpu::VertexBufferLayout {
                    array_stride: mem::size_of::<VertexData>() as u64,
                    step_mode: agpu::wgpu::VertexStepMode::Vertex,
                    attributes: &agpu::wgpu::vertex_attr_array![2 => Float32x2],
                },
            ])
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
        z: f32,
    ) {
        if type_id != &TypeId::of::<Quad>() {
            return;
        }

        let geometry: VertexBuffers<VertexData, u16> = {
            let rect = manager
                .get_rect(widget_id)
                .expect("widget does not have a rect");

            let clipping = manager.get_clipping(widget_id).try_get();

            let path = clipping.map_or_else(
                || {
                    let mut builder = Path::builder();

                    builder.add_rounded_rectangle(
                        &Rect {
                            origin: Point2D::new(rect.x, rect.y),
                            size: Size2D::new(rect.width, rect.height),
                        },
                        &BorderRadii {
                            top_left: 4.0,
                            top_right: 4.0,
                            bottom_left: 4.0,
                            bottom_right: 4.0,
                        },
                        Winding::Positive,
                    );

                    builder.build()
                },
                |clipping| Path::clone(&clipping),
            );

            // let path = {
            //     let mut builder = Path::builder();

            //     builder.add_rounded_rectangle(
            //         &Rect {
            //             origin: Point2D::new(rect.x, rect.y),
            //             size: Size2D::new(rect.width, rect.height),
            //         },
            //         &BorderRadii {
            //             top_left: 4.0,
            //             top_right: 4.0,
            //             bottom_left: 4.0,
            //             bottom_right: 4.0,
            //         },
            //         Winding::Positive,
            //     );

            //     let path = builder.build();

            //     match clipping
            //         .and_then(|clipping| ClippingMask::intersection(&path, clipping.as_ref()))
            //     {
            //         Some(path) => path,
            //         None => path,
            //     }
            // };

            let mut geometry = VertexBuffers::new();

            let mut tessellator = FillTessellator::new();
            {
                // Compute the tessellation.
                tessellator
                    .tessellate_path(
                        &path,
                        &FillOptions::default(),
                        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
                            VertexData(vertex.position().to_array())
                        }),
                    )
                    .unwrap();
            }

            geometry
        };

        let quad_data = ctx
            .gpu
            .new_buffer("agui_instance_buffer")
            .as_vertex_buffer()
            .create(bytemuck::bytes_of(&QuadData {
                z,
                color: manager
                    .get_as::<Quad>(widget_id)
                    .style
                    .as_ref()
                    .map_or(Color::default(), |style| style.color)
                    .as_rgba(),
            }));

        let vertex_data = ctx
            .gpu
            .new_buffer("agui_vertex_buffer")
            .as_vertex_buffer()
            .create(&geometry.vertices);

        let index_data = ctx
            .gpu
            .new_buffer("agui_index_buffer")
            .as_index_buffer()
            .create(&geometry.indices);

        self.widgets.insert(
            *widget_id,
            WidgetBuffer {
                quad_data,

                vertex_data,
                index_data,
                count: geometry.indices.len() as u32,
            },
        );
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
            r.set_vertex_buffer(0, widget_buffer.quad_data.slice(..))
                .set_vertex_buffer(1, widget_buffer.vertex_data.slice(..))
                .set_index_buffer(widget_buffer.index_data.slice(..))
                .draw_indexed(0..widget_buffer.count, 0, 0..1);
        }
    }
}
