use crate::{
    element::{Element, ElementId},
    listenable::EventBus,
    plugin::Plugins,
    util::tree::Tree,
};

pub struct PluginInitContext<'ctx> {
    pub bus: &'ctx EventBus,

    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,
}
