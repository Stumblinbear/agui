use std::{any::TypeId, collections::HashMap};

use crate::{
    context::WidgetContext,
    plugin::{event::WidgetEvent, WidgetPlugin},
    widget::{WidgetId, WidgetRef},
};

mod cache;
mod manager;
mod tree;

pub use manager::{WidgetManager, WidgetNode};

#[derive(Default)]
pub struct UI {
    plugins: HashMap<TypeId, Box<dyn WidgetPlugin>>,

    manager: WidgetManager,

    events: Vec<WidgetEvent>,
}

impl UI {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn get_manager(&self) -> &WidgetManager {
        &self.manager
    }

    pub const fn get_context(&self) -> &WidgetContext {
        self.manager.get_context()
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn add_plugin<P>(&mut self, plugin: P)
    where
        P: WidgetPlugin,
    {
        let type_id = TypeId::of::<P>();

        if self.plugins.insert(type_id, Box::new(plugin)).is_some() {
            log::warn!("plugin already exists, overwriting");
        }
    }

    pub fn get_plugin<P>(&mut self) -> Option<&P>
    where
        P: WidgetPlugin,
    {
        let type_id = TypeId::of::<P>();

        self.plugins
            .get(&type_id)
            .and_then(|plugin| plugin.downcast_ref())
    }

    pub fn set_root(&mut self, widget: WidgetRef) {
        self.manager.add(None, widget);
    }

    pub fn add(&mut self, parent_id: Option<WidgetId>, widget: WidgetRef) {
        self.manager.add(parent_id, widget);
    }

    pub fn remove(&mut self, widget_id: WidgetId) {
        self.manager.remove(widget_id);
    }

    /// Returns true of any events were fired
    pub fn update(&mut self) -> bool {
        self.manager.update(&mut self.events);

        if self.events.is_empty() {
            false
        } else {
            for event in self.events.drain(..) {
                for plugin in self.plugins.values_mut() {
                    plugin.on_event(&mut self.manager, &event);
                }
            }

            true
        }
    }
}
