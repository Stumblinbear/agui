use std::any::TypeId;

use agui_core::{
    element::ElementId,
    plugin::{
        context::{PluginMountContext, PluginUnmountContext},
        Plugin,
    },
    widget::{AnyWidget, ContextWidget},
};

use crate::{element::InheritedWidget, manager::InheritanceManager};

#[derive(Default)]
pub struct InheritancePlugin {
    manager: InheritanceManager,
}

impl Plugin for InheritancePlugin {
    fn on_mount(&mut self, ctx: PluginMountContext) {
        self.manager
            .create_node(ctx.get_parent_element_id(), ctx.get_element_id());
    }

    fn on_remount(&mut self, mut ctx: PluginMountContext) {
        let parent_scope_id = ctx.get_parent_element_id().and_then(|parent_element_id| {
            self.manager
                .get(parent_element_id)
                .expect("failed to get scope from parent")
                .get_scope()
        });

        let element_id = ctx.get_element_id();

        self.manager
            .update_inheritance_scope(&mut ctx, element_id, parent_scope_id);
    }

    fn on_unmount(&mut self, ctx: PluginUnmountContext) {
        self.manager.remove(ctx.get_element_id());
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
