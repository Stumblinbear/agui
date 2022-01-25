use std::cell::RefCell;

use agpu::{BindGroup, Buffer, GpuHandle, Sampler, Texture};
use agui::canvas::{font::FontId, texture::TextureId};
use glyph_brush_draw_cache::{ab_glyph::FontArc, DrawCache};

pub struct RenderContext {
    pub(crate) gpu: GpuHandle,

    pub(crate) render_size: Buffer,

    pub(crate) unknown_texture: Texture<agpu::D2>,
    pub(crate) texture_sampler: Sampler,

    pub(crate) textures: Vec<Texture<agpu::D2>>,

    pub(crate) fonts: Vec<FontArc>,
    pub(crate) font_texture: Texture<agpu::D2>,
    pub(crate) font_draw_cache: RefCell<DrawCache>,
}

impl RenderContext {
    pub fn get_texture(&self, texture_id: TextureId) -> Option<&Texture<agpu::D2>> {
        if let Some(texture_idx) = texture_id.idx() {
            if texture_idx < self.textures.len() {
                return Some(&self.textures[texture_idx]);
            }
        }

        None
    }

    pub fn load_texture(&mut self, texture: Texture<agpu::D2>) -> TextureId {
        self.textures.push(texture);

        TextureId::new(self.textures.len() - 1)
    }

    pub fn get_fonts(&self) -> &[FontArc] {
        &self.fonts
    }

    pub fn get_font(&self, font_id: FontId) -> Option<FontArc> {
        if let Some(font_idx) = font_id.idx() {
            if font_idx < self.fonts.len() {
                return Some(FontArc::clone(&self.fonts[font_idx]));
            }
        }

        None
    }

    pub fn load_font(&mut self, font: FontArc) -> FontId {
        self.fonts.push(font);

        FontId::new(self.fonts.len() - 1)
    }
}
