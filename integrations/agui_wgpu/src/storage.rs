use std::cell::RefCell;

use agui::{render::texture::TextureId, unit::Size};
use glyph_brush_draw_cache::DrawCache;

pub struct RenderStorage {
    pub render_size: wgpu::Buffer,

    pub unknown_texture_view: wgpu::TextureView,
    pub texture_sampler: wgpu::Sampler,

    pub textures: Vec<wgpu::TextureView>,

    pub font_texture: wgpu::Texture,
    pub font_texture_size: Size,
    pub font_texture_view: wgpu::TextureView,

    pub font_draw_cache: RefCell<DrawCache>,
}

impl RenderStorage {
    pub fn get_texture(&self, texture_id: TextureId) -> Option<&wgpu::TextureView> {
        if let Some(texture_idx) = texture_id.idx() {
            if texture_idx < self.textures.len() {
                return Some(&self.textures[texture_idx]);
            }
        }

        None
    }

    pub fn load_texture(&mut self, texture: wgpu::TextureView) -> TextureId {
        self.textures.push(texture);

        TextureId::new(self.textures.len() - 1)
    }
}
