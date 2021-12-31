use std::{any::TypeId, collections::HashMap, mem};

use agpu::{Buffer, Frame, GpuProgram, Texture};
use agui::{
    unit::Rect,
    widget::WidgetId,
    widgets::primitives::{FontArc, SectionGlyph, Text},
    WidgetManager,
};
use generational_arena::{Arena, Index as GenerationalIndex};
use glyph_brush_draw_cache::{CachedBy, DrawCache};

use super::{RenderContext, WidgetRenderPass};

const INITIAL_TEXTURE_SIZE: [u32; 2] = [1024, 1024];

const RECT_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const UV_BUFFER_SIZE: u64 = std::mem::size_of::<[f32; 4]>() as u64;
const GLYPH_BUFFER_SIZE: u64 = RECT_BUFFER_SIZE + UV_BUFFER_SIZE;

const PREALLOCATE: u64 = GLYPH_BUFFER_SIZE * 128;

pub struct TextRenderPass {
    fonts: Vec<FontArc>,

    draw_cache: DrawCache,

    texture: Texture,
    buffer: Buffer,

    locations: Arena<WidgetId>,
    widgets: HashMap<WidgetId, GenerationalIndex>,
    glyphs: HashMap<WidgetId, Vec<SectionGlyph>>,
}

impl TextRenderPass {
    pub fn new(program: &GpuProgram, ctx: &RenderContext) -> Self {
        Self {
            fonts: Vec::new(),

            draw_cache: DrawCache::builder().build(),

            texture: program
                .gpu
                .new_texture("TextRenderPass<Texture>")
                .create_empty(&INITIAL_TEXTURE_SIZE),
            buffer: program
                .gpu
                .new_buffer("TextRenderPass<Vertex>")
                .as_vertex_buffer()
                .allow_copy()
                .create_uninit(PREALLOCATE),

            locations: Arena::default(),
            widgets: HashMap::default(),
            glyphs: HashMap::default(),
        }
    }

    pub fn add_font(&mut self, font: FontArc) {
        self.fonts.push(font);
    }

    pub fn get_buffer_index(&self, widget_id: &WidgetId) -> Option<u64> {
        let index = match self.widgets.get(widget_id) {
            Some(widget) => widget,
            None => return None,
        };

        let index = index.into_raw_parts().0 as u64;

        Some(index * GLYPH_BUFFER_SIZE)
    }
}

impl WidgetRenderPass for TextRenderPass {
    fn added(
        &mut self,
        ctx: &RenderContext,
        manager: &WidgetManager,
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
            match self
                .draw_cache
                .cache_queued(&self.fonts, |rect, tex_data| {})
            {
                Ok(cached_by) => break cached_by,
                Err(_) => self.texture.resize(&[0, 0]),
            }
        };

        if let CachedBy::Reordering = cached_by {
            todo!();
        } else {
            let index = match self.get_buffer_index(widget_id) {
                Some(index) => index,
                None => return,
            };

            for (i, sg) in glyphs.iter().enumerate() {
                if let Some((tex_coords, px_coords)) =
                    self.draw_cache.rect_for(sg.font_id.0, &sg.glyph)
                {
                    ctx.gpu.queue.write_buffer(
                        &self.buffer,
                        index + i as u64,
                        bytemuck::cast_slice(&[
                            tex_coords.min.x,
                            tex_coords.min.y,
                            tex_coords.max.x,
                            tex_coords.max.y,
                            px_coords.min.x,
                            px_coords.min.y,
                            px_coords.max.x,
                            px_coords.max.y,
                        ]),
                    );
                }
            }

            self.glyphs.insert(*widget_id, glyphs);
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

        let index = self
            .widgets
            .remove(widget_id)
            .expect("removed nonexistent widget");

        self.locations.remove(index);

        self.glyphs.remove(widget_id);
    }

    fn update(&mut self, ctx: &RenderContext) {}

    fn render(&self, _ctx: &RenderContext, frame: &mut Frame) {}
}
