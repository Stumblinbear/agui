use std::collections::VecDeque;

use rustc_hash::FxHashSet;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    engine::{bindings::SharedSchedulerBinding, rendering::RenderManager, Dirty},
    util::tree::Tree,
};

pub struct RenderManagerBuilder<SB> {
    scheduler: SB,
}

impl Default for RenderManagerBuilder<()> {
    fn default() -> Self {
        Self { scheduler: () }
    }
}

impl RenderManagerBuilder<()> {
    pub fn with_scheduler<SB>(self, scheduler: SB) -> RenderManagerBuilder<SB>
    where
        SB: SharedSchedulerBinding,
    {
        RenderManagerBuilder { scheduler }
    }
}

impl<SB> RenderManagerBuilder<SB>
where
    SB: SharedSchedulerBinding,
{
    pub fn build(self) -> RenderManager<SB> {
        RenderManager {
            scheduler: self.scheduler,

            tree: Tree::default(),

            elements: SecondaryMap::default(),

            create_render_object: VecDeque::default(),
            update_render_object: FxHashSet::default(),
            forgotten_elements: FxHashSet::default(),

            needs_layout: Dirty::default(),
            needs_paint: Dirty::default(),

            cached_constraints: SecondaryMap::default(),

            layout_changed: SparseSecondaryMap::default(),

            needs_sync: SparseSecondaryMap::default(),
        }
    }
}
