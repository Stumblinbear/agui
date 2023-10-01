use std::rc::Rc;

use crate::widget::{
    element::{ElementUpdate, ElementWidget},
    AnyWidget, Widget,
};

use super::RenderView;

pub struct RenderViewElement {
    widget: Rc<RenderView>,
}

impl RenderViewElement {
    pub fn new(widget: Rc<RenderView>) -> Self {
        Self { widget }
    }

    pub fn get_child(&self) -> Widget {
        self.widget.child.clone()
    }
}

impl ElementWidget for RenderViewElement {
    fn widget_name(&self) -> &'static str {
        self.widget.widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        if let Some(new_widget) = new_widget.downcast::<RenderView>() {
            self.widget = new_widget;

            ElementUpdate::RebuildNecessary
        } else {
            ElementUpdate::Invalid
        }
    }
}

impl std::fmt::Debug for RenderViewElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("RenderViewElement");

        dbg.finish()
    }
}
