use crate::{
    engine::{event::ElementEvent, Engine},
    plugin::Plugin,
};

use super::RenderViewId;

pub trait Renderer: Plugin {
    type Target;

    fn create_context(
        &mut self,
        engine: &Engine,
        render_view_id: RenderViewId,
        target: &Self::Target,
        width: u32,
        height: u32,
    );

    fn remove_context(&mut self, engine: &Engine, render_view_id: RenderViewId);

    fn resize(&mut self, engine: &Engine, render_view_id: RenderViewId, width: u32, height: u32);

    fn redraw(&mut self, engine: &Engine, render_view_id: RenderViewId, events: &[ElementEvent]);

    fn render(&mut self, render_view_id: RenderViewId);
}
