use crate::{
    element::{ContextDirtyRenderObject, Element, ElementId},
    plugin::{
        context::{ContextPlugins, ContextPluginsMut},
        Plugins,
    },
    render::RenderObjectId,
    util::tree::Tree,
};

use super::{ContextElement, ContextElements, ContextRenderObject};

pub struct RenderObjectUpdateContext<'ctx> {
    pub plugins: &'ctx mut Plugins,

    pub element_tree: &'ctx Tree<ElementId, Element>,

    pub needs_layout: &'ctx mut bool,
    pub needs_paint: &'ctx mut bool,

    pub element_id: &'ctx ElementId,
    pub relayout_boundary_id: &'ctx Option<RenderObjectId>,
    pub render_object_id: &'ctx RenderObjectId,
}

impl<'ctx> ContextPlugins<'ctx> for RenderObjectUpdateContext<'ctx> {
    fn plugins(&self) -> &Plugins {
        self.plugins
    }
}

impl<'ctx> ContextPluginsMut<'ctx> for RenderObjectUpdateContext<'ctx> {
    fn plugins_mut(&mut self) -> &mut Plugins {
        self.plugins
    }
}

impl ContextElements for RenderObjectUpdateContext<'_> {
    fn elements(&self) -> &Tree<ElementId, Element> {
        self.element_tree
    }
}

impl ContextElement for RenderObjectUpdateContext<'_> {
    fn element_id(&self) -> ElementId {
        *self.element_id
    }
}

impl ContextRenderObject for RenderObjectUpdateContext<'_> {
    fn render_object_id(&self) -> RenderObjectId {
        *self.render_object_id
    }
}

impl ContextDirtyRenderObject for RenderObjectUpdateContext<'_> {
    fn mark_needs_layout(&mut self) {
        *self.needs_layout = true;
    }

    fn mark_needs_paint(&mut self) {
        *self.needs_paint = true;
    }
}
