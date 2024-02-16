use crate::{
    engine::rendering::{scheduler::RenderingScheduler, RenderingTree},
    render::RenderObjectId,
};

pub struct RenderingSpawnContext<'ctx> {
    pub scheduler: RenderingScheduler<'ctx>,

    pub parent_render_object_id: &'ctx Option<RenderObjectId>,
    pub render_object_id: &'ctx RenderObjectId,
}

pub struct RenderingUpdateContext<'ctx> {
    pub scheduler: RenderingScheduler<'ctx>,

    pub render_object_id: &'ctx RenderObjectId,
}

pub struct RenderingLayoutContext<'ctx> {
    pub tree: &'ctx mut RenderingTree,

    pub render_object_id: &'ctx RenderObjectId,
}
