use fnv::FnvHashSet;

use crate::{
    engine::tree::Tree,
    widget::{Widget, WidgetId},
};

pub struct PluginContext<'ctx> {
    pub(crate) tree: &'ctx Tree<WidgetId, Widget>,
    pub(crate) dirty: &'ctx mut FnvHashSet<WidgetId>,
}

impl PluginContext<'_> {
    pub fn get_tree(&self) -> &Tree<WidgetId, Widget> {
        self.tree
    }

    pub fn mark_dirty(&mut self, widget_id: WidgetId) {
        self.dirty.insert(widget_id);
    }
}
