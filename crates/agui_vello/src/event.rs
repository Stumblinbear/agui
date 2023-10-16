use crate::render::{VelloViewRendererHandle};
use agui_renderer::RenderViewId;
use std::sync::Arc;

pub enum VelloPluginEvent {
    ViewBind {
        render_view_id: RenderViewId,
        renderer: Arc<VelloViewRendererHandle>,
    },

    ViewUnbind {
        render_view_id: RenderViewId,
    },
}
