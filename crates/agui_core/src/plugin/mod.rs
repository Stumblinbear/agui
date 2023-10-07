use std::ops::{Deref, DerefMut};

use crate::unit::AsAny;

use self::context::{PluginMountContext, PluginUnmountContext};

pub mod context;

pub trait Plugin: AsAny {
    #[allow(unused_variables)]
    fn on_mount(&mut self, ctx: PluginMountContext) {}

    #[allow(unused_variables)]
    fn on_remount(&mut self, ctx: PluginMountContext) {}

    #[allow(unused_variables)]
    fn on_unmount(&mut self, ctx: PluginUnmountContext) {}
}

pub struct Plugins(Vec<Box<dyn Plugin>>);

impl Plugins {
    pub(crate) fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        Self(plugins)
    }

    pub fn get<P>(&self) -> Option<&P>
    where
        P: Plugin,
    {
        for plugin in self.0.iter() {
            if let Some(plugin) = plugin.as_ref().as_any().downcast_ref::<P>() {
                return Some(plugin);
            }
        }

        None
    }

    pub fn get_mut<P>(&mut self) -> Option<&mut P>
    where
        P: Plugin,
    {
        for plugin in self.0.iter_mut() {
            if let Some(plugin) = plugin.as_mut().as_any_mut().downcast_mut::<P>() {
                return Some(plugin);
            }
        }

        None
    }
}

impl Deref for Plugins {
    type Target = [Box<dyn Plugin>];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Plugins {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
