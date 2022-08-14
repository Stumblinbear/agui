use std::any::TypeId;

mod context;
mod element;
mod instance;
mod plugin_impl;
mod stateful;
mod stateless;

pub use self::{context::*, element::*, instance::*, plugin_impl::*, stateful::*, stateless::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginId(TypeId);

impl PluginId {
    pub fn of<P>() -> Self
    where
        P: PluginImpl,
    {
        Self(TypeId::of::<P>())
    }

    pub fn from(plugin: &BoxedPlugin) -> Self {
        Self(plugin.get_type_id())
    }
}

pub type BoxedPlugin = Box<dyn PluginInstance>;

pub trait IntoPlugin: std::fmt::Debug + 'static {
    fn into_plugin(self) -> BoxedPlugin;
}
