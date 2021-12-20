use std::collections::HashSet;

use crate::render::WidgetRenderer;

mod cache;
mod manager;
mod tree;
mod widget;

pub use manager::WidgetManager;
pub use widget::*;

pub struct UI<R>
where
    R: WidgetRenderer,
{
    manager: WidgetManager,
    renderer: R,

    added: HashSet<WidgetID>,
    removed: HashSet<WidgetID>,
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

    pub fn get_renderer(&self) -> &R {
        &self.renderer
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
        self.manager.update(&mut self.added, &mut self.removed);

        let did_change = (self.removed.len() + self.added.len()) != 0;

        for widget_id in self.removed.drain() {
            self.renderer.remove(&self.manager, widget_id);
        }

        for widget_id in self.added.drain() {
            self.renderer.create(&self.manager, widget_id);
        }

        // TODO: is it possible to limit the scope of layout refreshing?
        if did_change {
            self.renderer.refresh(&self.manager);
            true
        } else {
            false
        }
    }
}
