use agui_core::engine::Engine;

pub mod event;
pub mod global;
// pub mod hovering;
pub mod provider;
pub mod timeout;

pub trait DefaultPluginsExt {
    fn register_default_plugins(&mut self);
}

impl DefaultPluginsExt for Engine {
    fn register_default_plugins(&mut self) {
        self.add_plugin(global::GlobalPlugin);
        self.add_plugin(event::EventPlugin);
        self.add_plugin(provider::ProviderPlugin);
        // self.add_plugin(hovering::HoveringPlugin);
        self.add_plugin(timeout::TimeoutPlugin::default());
    }
}
