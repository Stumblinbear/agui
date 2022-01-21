use agui_core::engine::Engine;

pub mod hovering;
pub mod provider;
pub mod timeout;

pub trait DefaultPluginsExt {
    fn register_default_plugins(&mut self);
}

impl<'ui, Renderer, Picture> DefaultPluginsExt for Engine<'ui, Renderer, Picture>
where
    Renderer: agui_core::engine::render::Renderer<Picture>,
{
    fn register_default_plugins(&mut self) {
        self.init_plugin(hovering::HoveringPlugin);
        self.init_plugin(timeout::TimeoutPlugin);
    }
}
