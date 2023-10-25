use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use super::{
    context::{ContextPlugins, ContextPluginsMut},
    Plugin,
};

pub struct Plugins {
    inner: Box<dyn Plugin>,
}

impl Plugins {
    pub(crate) fn new<P>(plugins: P) -> Self
    where
        P: Plugin,
    {
        Self {
            inner: Box::new(plugins),
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
        self.inner.get(TypeId::of::<P>()).map(|plugin| {
            plugin
                .as_any()
                .downcast_ref()
                .expect("failed to downcast plugin")
        })
    }

    /// Returns the given plugin of type `P`.
    ///
    /// This will return `None` if it's not added to the engine, but also may return `None` if
    /// the plugin is currently being updated.
    pub fn get_mut<P>(&mut self) -> Option<&mut P>
    where
        P: Plugin,
    {
        self.inner.get_mut(TypeId::of::<P>()).map(|plugin| {
            plugin
                .as_any_mut()
                .downcast_mut()
                .expect("failed to downcast plugin")
        })
    }
}

impl ContextPlugins<'_> for Plugins {
    fn plugins(&self) -> &Plugins {
        self
    }
}

impl ContextPluginsMut<'_> for Plugins {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self
    }
}

impl Deref for Plugins {
    type Target = Box<dyn Plugin>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Plugins {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
