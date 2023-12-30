use crate::unit::{Font, Texture};

pub trait ViewBinding {
    fn request_visual_update(&self);

    fn request_semantics_update(&self);

    fn load_font(&self, font_data: &[u8]) -> Result<Font, Box<dyn std::error::Error>>;

    fn load_image(&self, image_data: &[u8]) -> Result<Texture, Box<dyn std::error::Error>>;
}
