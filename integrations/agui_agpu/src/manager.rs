use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
    mem,
    rc::{Rc, Weak},
    time::Instant,
};

use agpu::{
    wgpu::{self, TextureSampleType, TextureViewDimension},
    Frame, Gpu, RenderPipeline, TextureFormat,
};
use agui::{
    manager::WidgetManager,
    unit::Size,
    util::tree::{new_key_type, Tree},
    widget::WidgetId,
};
use glyph_brush_draw_cache::DrawCache;

use crate::{
    context::RenderContext,
    render::{
        builder::{shape::LayerShapeBuilder, text::TextDrawCallBuilder, DrawCallBuilder},
        canvas::RenderCanvas,
        data::{InstanceData, VertexData},
        draw_call::DrawCall,
        layer::RenderLayer,
    },
};

const INITIAL_FONT_CACHE_SIZE: (u32, u32) = (1024, 1024);

new_key_type! {
    pub struct LayerId;
}

pub struct RenderManager {
    pipeline: RenderPipeline,

    ctx: RenderContext,

    canvas_cache: HashMap<u64, Weak<RenderCanvas>>,
    draw_cache: HashMap<u64, Weak<DrawCall>>,

    widgets: HashMap<WidgetId, LayerId>,
    tree: Tree<LayerId, RenderLayer>,
}

impl RenderManager {
    pub fn new(gpu: &Gpu, size: Size) -> Self {
        const INSTANCE_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Instance,
            attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x2],
        };

        const VERTEX_LAYOUT: wgpu::VertexBufferLayout = wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<VertexData>() as u64,
            step_mode: agpu::wgpu::VertexStepMode::Vertex,
            attributes: &agpu::wgpu::vertex_attr_array![1 => Float32x4],
        };

        const VIEWPORT_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const OPTIONS_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const INDICES_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const POSITIONS_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 3,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        const TEXTURE_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 4,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };

        const SAMPLER_BINDING: wgpu::BindGroupLayoutEntry = wgpu::BindGroupLayoutEntry {
            binding: 5,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
            count: None,
        };

        let pipeline = gpu
            .new_pipeline("agui layer pipeline")
            .with_vertex(include_bytes!("shaders/layer.vert.spv"))
            .with_fragment(include_bytes!("shaders/layer.frag.spv"))
            .with_vertex_layouts(&[INSTANCE_LAYOUT, VERTEX_LAYOUT])
            .with_bind_groups(&[
                &gpu.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        VIEWPORT_BINDING,
                        OPTIONS_BINDING,
                        INDICES_BINDING,
                        POSITIONS_BINDING,
                        TEXTURE_BINDING,
                        SAMPLER_BINDING,
                    ],
                }),
            ])
            .create();

        Self {
            pipeline,

            ctx: RenderContext {
                gpu: Gpu::clone(gpu),

                render_size: gpu
                    .new_buffer("agui render size")
                    .as_uniform_buffer()
                    .allow_copy_to()
                    .create(&[size.width, size.height]),

                unknown_texture: gpu
                    .new_texture("agui unknown texture")
                    .allow_binding()
                    .create((1, 1), &[255_u8, 255, 255, 255]),
                texture_sampler: gpu.new_sampler("agui texture sampler").create(),

                textures: Vec::default(),

                font_texture: gpu
                    .new_texture("agui font texture")
                    .with_format(TextureFormat::R8Unorm)
                    .allow_binding()
                    .create_empty(INITIAL_FONT_CACHE_SIZE),

                font_draw_cache: RefCell::new(
                    DrawCache::builder()
                        .dimensions(INITIAL_FONT_CACHE_SIZE.0, INITIAL_FONT_CACHE_SIZE.1)
                        .build(),
                ),
            },

            canvas_cache: HashMap::default(),
            draw_cache: HashMap::default(),

            widgets: HashMap::default(),
            tree: Tree::default(),
        }
    }

    pub fn set_size(&mut self, size: Size) {
        self.ctx
            .render_size
            .write_unchecked(&[size.width, size.height]);
    }

    pub fn redraw(&mut self, manager: &WidgetManager) {
        let now = Instant::now();

        if let Some(root_id) = manager.get_widgets().get_root() {
            self.redraw_node(manager, root_id);
        } else if let Some(root_id) = self.tree.get_root() {
            self.canvas_cache.clear();
            self.draw_cache.clear();

            self.tree.remove(root_id, true);
        }

        tracing::info!("redrew in: {:?}", Instant::now().duration_since(now));
    }

    pub fn redraw_node(
        &mut self,
        manager: &WidgetManager,
        layer_id: Option<LayerId>,
        widget_id: WidgetId,
    ) {
        let fonts = manager.get_fonts();

        let tree = manager.get_widgets();

        tree.iter_down(Some(widget_id))
            .map(|widget_id| tree.get(widget_id).unwrap())
            .for_each(|node| {
                let widget = node.get().unwrap();

                let rect = match widget.get_rect() {
                    Some(rect) => rect,
                    None => return,
                };

                let mut canvas = Canvas::new(rect.into());

                widget.render(&mut canvas);

                let canvas_buffer = self.render_canvas(fonts, canvas);

                nodes.push(RenderNode {
                    pos: self
                        .ctx
                        .gpu
                        .new_buffer("agui layer instance buffer")
                        .as_vertex_buffer()
                        .create(&[rect.x, rect.y]),
                    canvas_buffer,
                });
            });

        self.root = root;

        // Remove any invalidated buffers from the cache
        self.canvas_cache
            .retain(|_, canvas_buffer| canvas_buffer.upgrade().is_some());

        self.draw_cache
            .retain(|_, draw_call| draw_call.upgrade().is_some());
    }

    fn render_canvas(
        &self,
        ctx: &mut RenderContext,
        layer: &mut Option<RenderLayer>,
        canvas: Canvas,
    ) -> Rc<RenderCanvas> {
        let mut hasher = DefaultHasher::new();
        canvas.hash(&mut hasher);
        let hash = hasher.finish();

        if let Some(canvas_buffer) = self
            .canvas_cache
            .get(&hash)
            .and_then(|canvas_buffer| canvas_buffer.upgrade())
        {
            return canvas_buffer;
        }

        let mut canvas_buffer = RenderCanvas::default();

        let mut layer_idx: usize = 0;

        let mut commands = Vec::new();
        let mut draw_call_builder: Option<Box<dyn DrawCallBuilder>> = None;

        for cmd in canvas.consume() {
            // Check if the current layer builder can process the command, and finalize the build if not
            if let Some(builder) = draw_call_builder.as_ref() {
                if !builder.can_process(&cmd) {
                    // Add the draw call to the current layer

                    let mut hasher = DefaultHasher::new();
                    commands.hash(&mut hasher);
                    let hash = hasher.finish();

                    if let Some(draw_call) = self
                        .draw_cache
                        .get(&hash)
                        .and_then(|draw_call| draw_call.upgrade())
                    {
                        canvas_buffer.nodes
                    }

                    if let Some(draw_call) = builder.build(ctx) {
                        canvas_buffer.nodes.push(draw_call);
                    }

                    commands.clear();
                    draw_call_builder = None;
                }
            }

            commands.push(cmd);

            match cmd {
                CanvasCommand::Layer {
                    rect,
                    shape,
                    anti_alias,
                    blend_mode,
                } => {
                    // Create a new layer and insert it after the current layer
                    let new_layer = Layer {
                        rect,
                        shape,

                        anti_alias,
                        blend_mode,

                        ..Layer::default()
                    };

                    canvas_buffer.layers.insert(layer_idx, new_layer);

                    // Switch to the new layer
                    layer_idx += 1;

                    continue;
                }

                CanvasCommand::Pop => {
                    // We can't pop beyond the layers owned by the canvas
                    if layer_idx == 0 {
                        panic!("can't pop layer, no layer to pop");
                    }

                    // Grab the previous layer
                    layer_idx -= 1;
                }

                CanvasCommand::Shape { .. } => {
                    if draw_call_builder.is_none() {
                        draw_call_builder =
                            Some(Box::new(LayerShapeBuilder::new(TextureId::default())));
                    }
                }

                CanvasCommand::Texture { texture_id, .. } => {
                    if draw_call_builder.is_none() {
                        draw_call_builder = Some(Box::new(LayerShapeBuilder::new(texture_id)));
                    }
                }

                CanvasCommand::Text { .. } => {
                    if draw_call_builder.is_none() {
                        draw_call_builder = Some(Box::new(TextDrawCallBuilder {
                            fonts: self.fonts,
                            ..TextDrawCallBuilder::default()
                        }));
                    }
                }

                cmd => {
                    tracing::error!("unknown command: {:?}", cmd);

                    continue;
                }
            }

            draw_call_builder.as_mut().unwrap().process(cmd);
        }

        if let Some(builder) = draw_call_builder.take() {
            canvas_buffer.layers[layer_idx]
                .draw_calls
                .extend(builder.build(ctx));
        }

        let canvas_buffer = Rc::new(canvas_buffer);

        self.canvas_cache
            .insert(hash, Rc::downgrade(&canvas_buffer));

        canvas_buffer
    }

    pub fn render(&self, mut frame: Frame) {
        frame
            .render_pass_cleared("agui clear pass", 0x11111111)
            .begin();

        let mut r = frame
            .render_pass("agui layer pass")
            .with_pipeline(&self.pipeline)
            .begin();

        for node in &self.nodes {
            r.set_vertex_buffer(0, node.pos.slice(..));

            for layer in &node.canvas_buffer.layers {
                for draw_call in &layer.draw_calls {
                    r.set_bind_group(0, &draw_call.bind_group, &[]);

                    r.set_vertex_buffer(1, draw_call.vertex_data.slice(..))
                        .draw(0..draw_call.count, 0..1);
                }
            }
        }
    }
}
