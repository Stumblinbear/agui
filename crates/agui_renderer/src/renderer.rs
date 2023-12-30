use std::sync::Arc;

use agui_core::unit::Size;

use crate::{RenderManifold, RenderViewId};

pub trait Renderer {
    type Target;

    fn bind(
        &self,
        render_view_id: RenderViewId,
        target: &Self::Target,
        size: Size,
    ) -> Result<Arc<dyn RenderManifold>, Box<dyn std::error::Error>>;
}
