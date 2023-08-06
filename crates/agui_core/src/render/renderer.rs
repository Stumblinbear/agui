use crate::{
    manager::{events::ElementEvent, WidgetManager},
    plugin::Plugin,
};

use super::RenderContextId;

pub trait Renderer: Plugin {
    type Target;

    fn create_context(
        &mut self,
        widget_manager: &WidgetManager,
        render_context_id: RenderContextId,
        target: &Self::Target,
        width: u32,
        height: u32,
    );

    fn remove_context(
        &mut self,
        widget_manager: &WidgetManager,
        render_context_id: RenderContextId,
    );

    fn resize(
        &mut self,
        widget_manager: &WidgetManager,
        render_context_id: RenderContextId,
        width: u32,
        height: u32,
    );

    fn redraw(
        &mut self,
        widget_manager: &WidgetManager,
        render_context_id: RenderContextId,
        events: &[ElementEvent],
    );

    fn render(&mut self, render_context_id: RenderContextId);
}
