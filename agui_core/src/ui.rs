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

    pub fn add<T>(&mut self, parent_id: Option<WidgetID>, widget: T)
    where
        T: Widget + 'static,
    {
        self.manager.add(parent_id, Box::new(widget));
    }

    pub fn remove(&mut self, widget_id: WidgetID) {
        self.manager.remove(widget_id);
    }

    pub fn update(&mut self) {
        self.manager.update(&mut self.added, &mut self.removed);

        for widget_id in self.removed.drain() {
            self.renderer.remove(&self.manager, widget_id);
        }

        for widget_id in self.added.drain() {
            self.renderer.create(&self.manager, widget_id);
        }
        
        // TODO: is it possible to limit the scope of layout refreshing?
        if self.added.len() > 0 || self.removed.len() > 0 {
            self.renderer.refresh(&self.manager);
        }
    }
}