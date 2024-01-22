use std::collections::VecDeque;

use rustc_hash::FxHashSet;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use crate::{
    engine::{
        bindings::RenderingSchedulerBinding, rendering::RenderManager,
        update_notifier::UpdateNotifier, Dirty,
    },
    util::tree::Tree,
};

pub struct RenderManagerBuilder<SB> {
    scheduler: SB,

    notifier: Option<UpdateNotifier>,
}

impl Default for RenderManagerBuilder<()> {
    fn default() -> Self {
        Self {
            scheduler: (),

            notifier: None,
        }
    }
}

impl RenderManagerBuilder<()> {
    pub fn with_scheduler<SB>(self, scheduler: SB) -> RenderManagerBuilder<SB>
    where
        SB: RenderingSchedulerBinding,
    {
        RenderManagerBuilder {
            scheduler,

            notifier: self.notifier,
        }
    }
}

impl<SB> RenderManagerBuilder<SB> {
    pub fn with_notifier(self, notifier: UpdateNotifier) -> RenderManagerBuilder<SB> {
        RenderManagerBuilder {
            scheduler: self.scheduler,

            notifier: Some(notifier),
        }
    }
}

impl<SB> RenderManagerBuilder<SB>
where
    SB: RenderingSchedulerBinding,
{
    pub fn build(self) -> RenderManager<SB> {
        let notifier = self.notifier.unwrap_or_else(|| UpdateNotifier::new().0);

        RenderManager {
            scheduler: self.scheduler,

            tree: Tree::default(),

            elements: SecondaryMap::default(),

            create_render_object: VecDeque::default(),
            update_render_object: FxHashSet::default(),
            forgotten_elements: FxHashSet::default(),

            needs_layout: Dirty::new(notifier.clone()),
            needs_paint: Dirty::new(notifier),

            cached_constraints: SecondaryMap::default(),

            layout_changed: SparseSecondaryMap::default(),

            needs_sync: SparseSecondaryMap::default(),
        }
    }
}
