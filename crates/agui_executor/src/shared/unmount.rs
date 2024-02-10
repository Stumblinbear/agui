use std::hash::BuildHasherDefault;

use agui_core::{
    element::{Element, ElementId, ElementUnmountContext},
    engine::{elements::strategies::UnmountElementStrategy, rendering::RenderingTree},
};
use rustc_hash::FxHasher;
use slotmap::SparseSecondaryMap;

pub struct ElementTreeUnmount<'cleanup> {
    pub rendering_tree: &'cleanup mut RenderingTree,

    pub updated_elements:
        &'cleanup mut SparseSecondaryMap<ElementId, (), BuildHasherDefault<FxHasher>>,
}

impl UnmountElementStrategy for ElementTreeUnmount<'_> {
    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn unmount(&mut self, mut ctx: ElementUnmountContext, element: Element) {
        self.rendering_tree.forget(*ctx.element_id);

        self.updated_elements.remove(*ctx.element_id);

        element.unmount(&mut ctx);
    }
}
