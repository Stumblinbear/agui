use std::{any::TypeId, collections::HashMap};

use agpu::{BindGroup, Buffer, Frame, GpuProgram, RenderPipeline, Sampler, Texture, TextureFormat};
use agui::{
    unit::Rect,
    widget::WidgetId,
    widgets::primitives::{FontArc, SectionGlyph, Text},
    WidgetManager,
};
use generational_arena::{Arena, Index as GenerationalIndex};
use glyph_brush_draw_cache::{CachedBy, DrawCache};

use super::{RenderContext, WidgetRenderPass};

const INITIAL_TEXTURE_SIZE: (u32, u32) = (1024, 1024);

const RECT_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const Z_BUFFER_SIZE: u64 = std::mem::size_of::<f32>() as u64;
const COLOR_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const UV_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;

const GLYPH_BUFFER_SIZE: u64 =
    RECT_BUFFER_SIZE + Z_BUFFER_SIZE + COLOR_BUFFER_SIZE + UV_BUFFER_SIZE;

const PREALLOCATE: u64 = GLYPH_BUFFER_SIZE * 128;

pub struct TextRenderPass {
    bind_group: BindGroup,
    pipeline: RenderPipeline,

    texture: Texture<agpu::D2>,
    sampler: Sampler,
    buffer: Buffer,

    draw_cache: DrawCache,

    fonts: Vec<FontArc>,

    locations: Arena<SectionGlyph>,
    widgets: HashMap<WidgetId, Vec<GenerationalIndex>>,
}

impl TextRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        let texture = program
            .gpu
            .new_texture("agui_text_texture")
            .with_format(TextureFormat::R8Unorm)
            .allow_binding()
            .create_empty(INITIAL_TEXTURE_SIZE);

        let sampler = program.gpu.new_sampler("agui_text_sampler").create();

        let bindings = &[
            ctx.bind_app_settings(),
            texture.bind_texture(),
            sampler.bind(),
        ];

        let bind_group = program.gpu.create_bind_group(bindings);

        let pipeline = program
            .gpu
            .new_pipeline("agui_text_pipeline")
            .with_vertex(include_bytes!("shader/text.vert.spv"))
            .with_fragment(include_bytes!("shader/text.frag.spv"))
            .with_vertex_layouts(&[agpu::wgpu::VertexBufferLayout {
                array_stride: GLYPH_BUFFER_SIZE,
                step_mode: agpu::wgpu::VertexStepMode::Instance,
                attributes: &agpu::wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32, 2 => Float32x4, 3 => Float32x4],
            }])
            .with_bind_groups(&[&bind_group.layout])
            .create();

        Self {
            bind_group,
            pipeline,

            texture,
            sampler,
            buffer: program
                .gpu
                .new_buffer("agui_text_buffer")
                .as_vertex_buffer()
                .allow_copy()
                .create_uninit(PREALLOCATE),

            draw_cache: DrawCache::builder()
                .dimensions(INITIAL_TEXTURE_SIZE.0, INITIAL_TEXTURE_SIZE.1)
                .build(),

            fonts: Vec::new(),

            locations: Arena::default(),
            widgets: HashMap::default(),
        }
    }

    pub fn add_font(&mut self, font: FontArc) {
        self.fonts.push(font);
    }
}

impl WidgetRenderPass for TextRenderPass {
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
    }

    fn layout(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
        rect: &Rect,
    ) {
        if type_id != &TypeId::of::<Text>() {
            return;
        }

        let text = manager.get_as::<Text>(widget_id);

        let glyphs = text.get_glyphs(&self.fonts, (rect.width, rect.height));

        for sg in &glyphs {
            self.draw_cache.queue_glyph(sg.font_id.0, sg.glyph.clone());
        }

        let cached_by = loop {
            match self.draw_cache.cache_queued(&self.fonts, |rect, tex_data| {
                self.texture.write_block(
                    (rect.min[0], rect.min[1]),
                    (rect.width(), rect.height()),
                    tex_data,
                );
            }) {
                Ok(cached_by) => break cached_by,
                Err(_) => {
                    let size = self.texture.size;

                    self.texture.resize((size.0 + 32, size.1 + 32));
                }
            }
        };

        if let CachedBy::Reordering = cached_by {
            todo!();
        } else {
            if let Some(widget_glyphs) = self.widgets.remove(widget_id) {
                for index in widget_glyphs {
                    self.locations.remove(index);
                }
            }

            let mut widget_glyphs = Vec::with_capacity(glyphs.len());

            for (i, sg) in glyphs.into_iter().enumerate() {
                if let Some((tex_coords, px_coords)) =
                    self.draw_cache.rect_for(sg.font_id.0, &sg.glyph)
                {
                    let index = self.locations.insert(sg);

                    widget_glyphs.push(index);

                    ctx.gpu.queue.write_buffer(
                        &self.buffer,
                        (index.into_raw_parts().0 + i) as u64 * GLYPH_BUFFER_SIZE,
                        bytemuck::cast_slice(&[
                            rect.x + px_coords.min.x,
                            rect.y + px_coords.min.y,
                            rect.x + px_coords.max.x,
                            rect.y + px_coords.max.y,
                            0.0,
                            tex_coords.min.x,
                            tex_coords.min.y,
                            tex_coords.max.x,
                            tex_coords.max.y,
                            1.0,
                            1.0,
                            0.0,
                            0.0,
                        ]),
                    );
                }
            }

            self.widgets.insert(*widget_id, widget_glyphs);
        }
    }

    fn removed(
        &mut self,
        _ctx: &RenderContext,
        _manager: &WidgetManager,
        type_id: &TypeId,
        widget_id: &WidgetId,
    ) {
        if type_id != &TypeId::of::<Text>() {
            return;
        }

        if let Some(widget_glyphs) = self.widgets.remove(widget_id) {
            for index in widget_glyphs {
                self.locations.remove(index);
            }
        }
    }

    fn update(&mut self, _ctx: &RenderContext) {}

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {
        let mut r = frame
            .render_pass("agui_text_pass")
            .with_pipeline(&self.pipeline)
            .begin();

        r.set_bind_group(0, &self.bind_group, &[]);

        r.set_vertex_buffer(0, self.buffer.slice(..))
            .draw(0..6, 0..(self.locations.capacity() as u32));
    }
}
