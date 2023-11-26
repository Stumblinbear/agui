use agui_core::render::binding::RenderBinding;

pub struct VelloRenderBinding {}

impl RenderBinding for VelloRenderBinding {
    fn request_visual_update(&self) {
        todo!()
    }

    fn request_semantics_update(&self) {
        todo!()
    }

    fn load_font(
        &self,
        font_data: &[u8],
    ) -> Result<agui_core::unit::Font, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }

    fn load_image(
        &self,
        image_data: &[u8],
    ) -> Result<agui_core::unit::Texture, Box<dyn std::error::Error + Send + Sync>> {
        todo!()
    }
}
