use std::collections::HashSet;

use crate::{
    context::WidgetContext,
    render::{WidgetChanged, WidgetRenderer},
    widget::{WidgetID, WidgetRef},
};

mod cache;
mod manager;
mod tree;

pub use manager::{WidgetManager, WidgetNode};

pub struct UI<R>
where
    R: WidgetRenderer,
{
    manager: WidgetManager,
    renderer: R,

    added: HashSet<WidgetChanged>,
    removed: HashSet<WidgetChanged>,
}

impl<R> UI<R>
where
    R: WidgetRenderer,
{
    #[must_use]
    pub fn init() -> Self
    where
        R: Default,
    {
        Self::new(R::default())
    }

    pub fn new(renderer: R) -> Self {
        Self {
            manager: WidgetManager::new(),
            renderer,

            added: HashSet::default(),
            removed: HashSet::default(),
        }
    }

    pub fn get_manager(&self) -> &WidgetManager {
        &self.manager
    }

    pub fn get_context(&self) -> &WidgetContext {
        self.manager.get_context()
    }

    pub fn get_renderer(&self) -> &R {
        &self.renderer
    }

    pub fn get_renderer_mut(&mut self) -> &mut R {
        &mut self.renderer
    }

    pub fn set_root(&mut self, widget: WidgetRef) {
        self.manager.add(None, widget);
    }

    pub fn add(&mut self, parent_id: Option<WidgetID>, widget: WidgetRef) {
        self.manager.add(parent_id, widget);
    }

    pub fn remove(&mut self, widget_id: WidgetID) {
        self.manager.remove(widget_id);
    }

    /// Returns true of any element in the tree was changed
    pub fn update(&mut self) -> bool {
        let changed = self.manager.update(&mut self.added, &mut self.removed);

        let did_change = (self.removed.len() + self.added.len() + changed.len()) != 0;

        self.removed
            .drain()
            .for_each(|widget| self.renderer.removed(&self.manager, widget));

        self.added
            .drain()
            .for_each(|widget| self.renderer.added(&self.manager, widget));

        for widget in changed {
            self.renderer.refresh(&self.manager, widget);
        }
        
        did_change
    }
}
