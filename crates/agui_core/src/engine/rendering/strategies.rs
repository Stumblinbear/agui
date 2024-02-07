use crate::{
    element::ElementId,
    engine::rendering::{
        context::{RenderingLayoutContext, RenderingSpawnContext, RenderingUpdateContext},
        view::View,
    },
    render::{object::RenderObject, RenderObjectId},
};

pub trait RenderingTreeCreateStrategy {
    fn create(&mut self, ctx: RenderingSpawnContext, element_id: ElementId) -> RenderObject;

    fn create_view(&mut self, element_id: ElementId) -> Option<Box<dyn View + Send>>;
}

pub trait RenderingTreeUpdateStrategy {
    fn get_children(&self, element_id: ElementId) -> &[ElementId];

    fn update(
        &mut self,
        ctx: RenderingUpdateContext,
        element_id: ElementId,
        render_object: &mut RenderObject,
    );
}

pub trait RenderingTreeCleanupStrategy {
    #[allow(unused_variables)]
    fn on_removed(&mut self, render_object_id: RenderObjectId) {}
}

pub trait RenderingTreeLayoutStrategy {
    #[allow(unused_variables)]
    fn on_constraints_changed(
        &mut self,
        ctx: RenderingLayoutContext,
        render_object: &RenderObject,
    ) {
    }

    #[allow(unused_variables)]
    fn on_size_changed(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {}

    #[allow(unused_variables)]
    fn on_offset_changed(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {}

    #[allow(unused_variables)]
    fn on_laid_out(&mut self, ctx: RenderingLayoutContext, render_object: &RenderObject) {}
}
