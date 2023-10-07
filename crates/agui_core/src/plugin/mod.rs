use crate::unit::AsAny;

use self::context::{PluginMountContext, PluginUnmountContext};

pub mod context;

pub trait Plugin: AsAny {
    fn on_mount(&mut self, ctx: PluginMountContext) {}

    fn on_remount(&mut self, ctx: PluginMountContext) {}

    fn on_unmount(&mut self, ctx: PluginUnmountContext) {}
}

pub struct Plugins<'ctx>(&'ctx mut [Box<dyn Plugin>]);

impl<'ctx> Plugins<'ctx> {
    pub(crate) fn new(plugins: &'ctx mut [Box<dyn Plugin>]) -> Self {
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
