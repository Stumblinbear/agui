use std::ops::{Deref, DerefMut};

use crate::unit::AsAny;

use self::context::{
    PluginAfterUpdateContext, PluginBeforeUpdateContext, PluginElementBuildContext,
    PluginElementMountContext, PluginElementRemountContext, PluginElementUnmountContext,
    PluginInitContext,
};

pub mod context;

#[allow(unused_variables)]
pub trait Plugin: AsAny {
    fn capabilities(&self) -> Capabilities;

    /// Called when the engine is initialized.
    fn on_init(&mut self, ctx: PluginInitContext) {}

    /// Called before each engine update, before any changes have been processed.
    fn on_before_update(&mut self, ctx: PluginBeforeUpdateContext) {}

    /// Called after each engine update, after all changes have been processed and the tree
    /// has settled.
    fn on_after_update(&mut self, ctx: PluginAfterUpdateContext) {}

    fn on_element_mount(&mut self, ctx: PluginElementMountContext) {}

    fn on_element_remount(&mut self, ctx: PluginElementRemountContext) {}

    fn on_element_unmount(&mut self, ctx: PluginElementUnmountContext) {}

    fn on_element_build(&mut self, ctx: PluginElementBuildContext) {}
}

pub struct Plugins {
    inner: Vec<PluginObject>,

    plugin_in_use: Option<PluginObject>,
}

impl Plugins {
    pub(crate) fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        Self {
            inner: plugins.into_iter().map(PluginObject::new).collect(),

            plugin_in_use: Some(PluginObject::new(Box::new(PlaceholderPlugin))),
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
            if let Some(plugin) = (*plugin).as_any().downcast_ref::<P>() {
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
            if let Some(plugin) = (*plugin).as_any_mut().downcast_mut::<P>() {
                return Some(plugin);
            }
        }

        None
    }

    pub(crate) fn with<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Self, &mut PluginObject),
    {
        let num_plugins = self.inner.len();

        if num_plugins == 0 {
            return;
        }

        let mut plugin_in_use = self
            .plugin_in_use
            .take()
            .expect("cannot use plugins while they are being iterated over");

        for i in 0..num_plugins {
            std::mem::swap(&mut self.inner[i], &mut plugin_in_use);

            f(self, &mut plugin_in_use);

            std::mem::swap(&mut self.inner[i], &mut plugin_in_use);
        }

        self.plugin_in_use = Some(plugin_in_use);
    }
}

pub struct PluginObject {
    pub inner: Box<dyn Plugin>,

    pub capabilities: Capabilities,
}

impl PluginObject {
    pub fn new(plugin: Box<dyn Plugin>) -> Self {
        Self {
            inner: plugin,

            capabilities: Capabilities::default(),
        }
    }
}

impl Deref for PluginObject {
    type Target = dyn Plugin;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl DerefMut for PluginObject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut()
    }
}

bitflags::bitflags! {
    #[derive(Default)]
    pub struct Capabilities: u8 {
        const BEFORE_UPDATE = 0b0000_0001;
        const AFTER_UPDATE  = 0b0000_0010;

        const ELEMENT_MOUNT  = 0b0000_0100;
        const ELEMENT_UNMOUNT  = 0b0000_1000;
        const ELEMENT_BUILD  = 0b0001_0000;

        const ELEMENT_LIFECYCLE = Capabilities::ELEMENT_MOUNT.bits() | Capabilities::ELEMENT_UNMOUNT.bits() | Capabilities::ELEMENT_BUILD.bits();
    }
}

/// Exists as a stand-in for plugins that are currently being iterated over.
struct PlaceholderPlugin;

impl Plugin for PlaceholderPlugin {
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
}
