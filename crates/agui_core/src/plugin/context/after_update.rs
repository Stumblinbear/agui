use crate::{
    element::{Element, ElementId},
    engine::event::ElementEvent,
    plugin::Plugins,
    util::tree::Tree,
};

pub struct PluginAfterUpdateContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub events: &'ctx [ElementEvent],
}
