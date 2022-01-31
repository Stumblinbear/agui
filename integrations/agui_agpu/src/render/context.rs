use std::cell::RefCell;

use agpu::{Buffer, GpuHandle, Sampler, Texture};
use agui::canvas::texture::TextureId;
use glyph_brush_draw_cache::DrawCache;

pub struct RenderContext {
    pub(crate) gpu: GpuHandle,

    pub(crate) render_size: Buffer,

    pub(crate) unknown_texture: Texture<agpu::D2>,
    pub(crate) texture_sampler: Sampler,

    pub(crate) textures: Vec<Texture<agpu::D2>>,

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
}
