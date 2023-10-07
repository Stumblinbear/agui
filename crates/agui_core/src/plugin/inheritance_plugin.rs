use std::any::TypeId;

use crate::{
    element::ElementId,
    inheritance::manager::InheritanceManager,
    widget::{AnyWidget, InheritedWidget},
};

use super::{
    context::{PluginMountContext, PluginUnmountContext},
    Plugin,
};

#[derive(Default)]
pub struct InheritancePlugin {
    manager: InheritanceManager,
}

impl Plugin for InheritancePlugin {
    fn on_mount(&mut self, ctx: PluginMountContext) {
        self.manager
            .create_node(ctx.parent_element_id, ctx.element_id);
    }

    fn on_remount(&mut self, ctx: PluginMountContext) {
        let parent_scope_id = ctx.parent_element_id.and_then(|parent_element_id| {
            self.manager
                .get(parent_element_id)
                .expect("failed to get scope from parent")
                .get_scope()
        });

        self.manager.update_inheritance_scope(
            ctx.element_tree,
            ctx.dirty,
            ctx.element_id,
            parent_scope_id,
        );
    }

    fn on_unmount(&mut self, ctx: PluginUnmountContext) {
        self.manager.remove(ctx.element_id);
    }
}

impl InheritancePlugin {
    pub fn depend_on_inherited_element<I>(&mut self, element_id: ElementId) -> Option<ElementId>
    where
        I: AnyWidget + InheritedWidget,
    {
        self.manager
            .depend_on_inherited_element(element_id, TypeId::of::<I>())
    }

    pub fn iter_listeners(
        &self,
        element_id: ElementId,
    ) -> Option<impl Iterator<Item = ElementId> + '_> {
        self.manager
            .get_as_scope(element_id)
            .map(|scope| scope.iter_listeners())
    }

    pub(crate) fn create_scope(
        &mut self,
        parent_element_id: Option<ElementId>,
        element_id: ElementId,
        type_id: TypeId,
    ) {
        self.manager
            .create_scope(type_id, parent_element_id, element_id);
    }
}
