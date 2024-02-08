use agui_core::{
    element::{ElementId, RenderObjectUpdateContext},
    engine::{
        elements::ElementTree,
        rendering::{context::RenderingUpdateContext, strategies::RenderingTreeUpdateStrategy},
    },
    render::{object::RenderObject, RenderObjectId},
};
use rustc_hash::FxHashSet;
use slotmap::SparseSecondaryMap;

use crate::local::scheduler::LocalScheduler;

pub struct ImmediatelyUpdateRenderObjects<'update> {
    pub scheduler: &'update mut LocalScheduler,

    pub element_tree: &'update ElementTree,

    pub needs_layout: &'update mut SparseSecondaryMap<RenderObjectId, ()>,
    pub needs_paint: &'update mut FxHashSet<RenderObjectId>,
}

impl RenderingTreeUpdateStrategy for ImmediatelyUpdateRenderObjects<'_> {
    fn get_children(&self, element_id: ElementId) -> &[ElementId] {
        self.element_tree
            .as_ref()
            .get_children(element_id)
            .expect("element missing while updating render object")
    }

    #[tracing::instrument(level = "debug", skip(self, ctx))]
    fn update(
        &mut self,
        ctx: RenderingUpdateContext,
        element_id: ElementId,
        render_object: &mut RenderObject,
    ) {
        let mut needs_layout = false;
        let mut needs_paint = false;

        self.element_tree
            .as_ref()
            .get(element_id)
            .expect("element missing while updating render object")
            .update_render_object(
                &mut RenderObjectUpdateContext {
                    scheduler: &mut ctx.scheduler.with_strategy(self.scheduler),

                    needs_layout: &mut needs_layout,
                    needs_paint: &mut needs_paint,

                    render_object_id: ctx.render_object_id,
                },
                render_object,
            );

        if needs_layout {
            self.needs_layout.insert(*ctx.render_object_id, ());
        } else if needs_paint {
            self.needs_paint.insert(*ctx.render_object_id);
        }
    }
}
