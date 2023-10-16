use std::sync::Arc;

use agui_core::unit::Size;

use crate::RenderViewId;

mod widget;

pub use widget::*;

pub trait Renderer {
    type Target;

    fn bind(
        &self,
        render_view_id: RenderViewId,
        target: &Self::Target,
        size: Size,
    ) -> Result<Arc<dyn ViewRenderer>, Box<dyn std::error::Error>>;
}

pub trait ViewRenderer {
    fn resize(&self, size: Size);

    fn render(&self);
}
