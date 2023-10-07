use agui_core::engine::{event::ElementEvent, Engine};

mod element;
mod manager;
mod plugin;
mod view;
mod widget;

pub use plugin::*;
pub use view::*;
pub use widget::*;

pub trait Renderer<T> {
    fn create_view(
        &mut self,
        engine: &Engine,
        render_view_id: RenderViewId,
        target: &T,
        width: u32,
        height: u32,
    );

    fn remove_view(&mut self, engine: &Engine, render_view_id: RenderViewId);

    fn resize(&mut self, engine: &Engine, render_view_id: RenderViewId, width: u32, height: u32);

    fn redraw(&mut self, engine: &Engine, render_view_id: RenderViewId, events: &[ElementEvent]);

    fn render(&mut self, render_view_id: RenderViewId);
}
