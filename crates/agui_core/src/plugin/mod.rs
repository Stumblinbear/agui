use crate::unit::AsAny;

use self::context::{
    PluginAfterUpdateContext, PluginBeforeUpdateContext, PluginElementMountContext,
    PluginElementUnmountContext,
};

pub mod context;

pub trait Plugin: AsAny {
    /// Called before each engine update, before any changes have been processed.
    #[allow(unused_variables)]
    fn on_before_update(&mut self, ctx: PluginBeforeUpdateContext) {}

    /// Called after each engine update, after all changes have been processed and the tree
    /// has settled.
    #[allow(unused_variables)]
    fn on_after_update(&mut self, ctx: PluginAfterUpdateContext) {}

    #[allow(unused_variables)]
    fn on_element_mount(&mut self, ctx: PluginElementMountContext) {}

    #[allow(unused_variables)]
    fn on_element_remount(&mut self, ctx: PluginElementMountContext) {}

    #[allow(unused_variables)]
    fn on_element_unmount(&mut self, ctx: PluginElementUnmountContext) {}
}

pub struct Plugins {
    inner: Vec<Box<dyn Plugin>>,

    plugin_in_use: Option<Box<dyn Plugin>>,
}

impl Plugins {
    pub(crate) fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        Self {
            inner: plugins,

            plugin_in_use: Some(Box::new(PlaceholderPlugin)),
        }
    }

    /// Returns the given plugin of type `P`.
    ///
    /// This will return `None` if it's not added to the engine, but also may return `None` if
    /// the plugin is currently being updated.
    pub fn get<P>(&self) -> Option<&P>
    where
        P: Plugin,
    {
        for plugin in self.inner.iter() {
            if let Some(plugin) = plugin.as_ref().as_any().downcast_ref::<P>() {
                return Some(plugin);
            }
        }

        None
    }

    /// Returns the given plugin of type `P`.
    ///
    /// This will return `None` if it's not added to the engine, but also may return `None` if
    /// the plugin is currently being updated.
    pub fn get_mut<P>(&mut self) -> Option<&mut P>
    where
        P: Plugin,
    {
        for plugin in self.inner.iter_mut() {
            if let Some(plugin) = plugin.as_mut().as_any_mut().downcast_mut::<P>() {
                return Some(plugin);
            }
        }

        None
    }

    pub(crate) fn with<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Self, &mut dyn Plugin),
    {
        for i in 0..self.inner.len() {
            let mut plugin_in_use = self
                .plugin_in_use
                .take()
                .expect("can only have one plugin in use at any given time");

            std::mem::swap(&mut self.inner[i], &mut plugin_in_use);

            f(self, plugin_in_use.as_mut());

            std::mem::swap(&mut self.inner[i], &mut plugin_in_use);

            self.plugin_in_use = Some(plugin_in_use);
        }
    }
}

/// Exists as a stand-in for plugins that are currently being iterated over.
struct PlaceholderPlugin;

impl Plugin for PlaceholderPlugin {}
