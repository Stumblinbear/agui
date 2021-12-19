use std::collections::HashSet;

use crate::{
    render::WidgetRenderer,
    widget::{Widget, WidgetID},
};

mod layout;
mod manager;
mod tree;

pub use manager::WidgetManager;

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
    pub fn init() -> UI<R>
    where
        R: Default,
    {
        UI::new(R::default())
    }

    pub fn new(renderer: R) -> UI<R> {
        UI {
            manager: WidgetManager::new(),
            renderer,

            added: Default::default(),
            removed: Default::default(),
        }
    }

    pub fn get_renderer(&self) -> &R {
        &self.renderer
    }

    pub fn set_root<T>(&mut self, widget: T)
    where
        T: Widget + 'static,
    {
        self.manager.add(None, Box::new(widget));
    }

    pub fn add<T>(&mut self, parent_id: Option<WidgetID>, widget: T)
    where
        T: Widget + 'static,
    {
        self.manager.add(parent_id, Box::new(widget));
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
            return true;
        }else{
            return false;
        }
    }
}