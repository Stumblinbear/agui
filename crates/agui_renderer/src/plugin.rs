use agui_core::{
    element::{ContextElement, ContextElements},
    plugin::{
        context::{
            PluginElementMountContext, PluginElementRemountContext, PluginElementUnmountContext,
        },
        Plugin,
    },
};

use crate::manager::RenderViewManager;

#[derive(Default)]
pub struct RenderViewPlugin {
    pub(crate) manager: RenderViewManager,
}

impl Plugin for RenderViewPlugin {
    fn on_element_mount(&mut self, ctx: &mut PluginElementMountContext) {
        self.manager.add(ctx.parent_element_id(), ctx.element_id());
    }

    fn on_element_remount(&mut self, ctx: &mut PluginElementRemountContext) {
        let element_id = ctx.element_id();

        let parent_render_view_id = ctx
            .parent_element_id()
            .and_then(|element_id| self.manager.get_view(element_id));

        self.manager
            .update_render_view(ctx.elements(), element_id, parent_render_view_id);
    }

    fn on_element_unmount(&mut self, ctx: &mut PluginElementUnmountContext) {
        // if let Some(render_view_id) = self.manager.get_view(*ctx.element_id) {
        //     let view = self
        //         .manager
        //         .get_entry(render_view_id)
        //         .expect("render view missing during unmount");

        //     if view.boundary_element_id == *ctx.element_id {
        //         // If the element that was unmounted is the boundary then we have to unmount all children
        //         // first, since the children will be unmounted after the boundary has already been removed
        //         // from the manager meaning they'd no longer have access to the binding that was used to
        //         // attach them.
        //         for render_object_id in ctx
        //             .element_tree
        //             .iter_subtree(view.boundary_element_id, |element_id| {
        //                 self.manager.get_view(element_id) == Some(render_view_id)
        //             })
        //             .filter_map(|element_id| ctx.element_tree.get(element_id))
        //             .filter_map(|element| element.render_object_id())
        //         {
        //             self.events
        //                 .push_back(RenderViewEvent::Detatch(render_object_id, view.binding));
        //         }
        //     } else if let Some(render_object_id) = ctx.element.render_object_id() {
        //         self.events
        //             .push_back(RenderViewEvent::Detatch(render_object_id, view.binding));
        //     }

        //     self.manager.remove(*ctx.element_id);
        // }

        // self.forgotten_render_objects
        //     .extend(ctx.element.render_object_id())

        self.manager.remove(*ctx.element_id);
    }

    // fn on_element_build(&mut self, ctx: &mut PluginElementBuildContext) {}

    // fn on_create_render_object(&mut self, ctx: &mut PluginCreateRenderObjectContext) {
    //     if let Some(render_view_id) = self.manager.get_view(*ctx.element_id) {
    //         let binding = self
    //             .manager
    //             .get_binding(render_view_id)
    //             .expect("render view missing while creating render object");

    //         self.events.push_back(RenderViewEvent::Attach(
    //             *ctx.render_object_id,
    //             Rc::clone(&binding),
    //         ));
    //     }
    // }

    // fn on_after_update(&mut self, ctx: &mut PluginAfterUpdateContext) {
    //     while let Some(event) = self.events.pop_front() {
    //         match event {
    //             RenderViewEvent::Attach(render_object_id, binding) => {
    //                 if self.forgotten_render_objects.remove(&render_object_id) {
    //                     continue;
    //                 }

    //                 ctx.render_object_tree
    //                     .get(render_object_id)
    //                     .expect("render object missing")
    //                     .attach(binding);
    //             }

    //             RenderViewEvent::Detatch(render_object_id, binding) => {
    //                 if self.forgotten_render_objects.remove(&render_object_id) {
    //                     continue;
    //                 }

    //                 ctx.render_object_tree
    //                     .get(render_object_id)
    //                     .expect("render object missing")
    //                     .detach(binding);
    //             }
    //         }
    //     }
    // }
}
