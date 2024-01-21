use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    engine::{
        bindings::{ElementBinding, LocalSchedulerBinding},
        update_notifier::UpdateNotifier,
        widgets::WidgetManager,
        Dirty,
    },
    inheritance::InheritanceManager,
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

pub struct WidgetManagerBuilder<EB, SB, const HAS_ROOT: bool> {
    element_binding: EB,
    scheduler: SB,

    root: Option<Widget>,
}

impl Default for WidgetManagerBuilder<(), (), false> {
    fn default() -> Self {
        Self {
            element_binding: (),
            scheduler: (),

            root: None,
        }
    }
}

impl<SB, const HAS_ROOT: bool> WidgetManagerBuilder<(), SB, HAS_ROOT> {
    pub fn with_element_binding<EB>(
        self,
        element_binding: EB,
    ) -> WidgetManagerBuilder<EB, SB, HAS_ROOT>
    where
        EB: ElementBinding,
    {
        WidgetManagerBuilder {
            element_binding,
            scheduler: self.scheduler,

            root: self.root,
        }
    }
}

impl<EB, const HAS_ROOT: bool> WidgetManagerBuilder<EB, (), HAS_ROOT> {
    pub fn with_scheduler<SB>(self, scheduler: SB) -> WidgetManagerBuilder<EB, SB, HAS_ROOT>
    where
        SB: LocalSchedulerBinding,
    {
        WidgetManagerBuilder {
            element_binding: self.element_binding,
            scheduler,

            root: self.root,
        }
    }
}

impl<EB, SB> WidgetManagerBuilder<EB, SB, false> {
    pub fn with_root<W>(self, root: W) -> WidgetManagerBuilder<EB, SB, true>
    where
        W: IntoWidget,
    {
        WidgetManagerBuilder {
            element_binding: self.element_binding,
            scheduler: self.scheduler,

            root: Some(root.into_widget()),
        }
    }
}

impl<EB, SB> WidgetManagerBuilder<EB, SB, true>
where
    EB: ElementBinding,
    SB: LocalSchedulerBinding,
{
    pub fn build(self) -> WidgetManager<EB, SB> {
        let (callback_tx, callback_rx) = mpsc::channel();
        let notifier = UpdateNotifier::new();

        let mut manager = WidgetManager {
            element_binding: self.element_binding,
            scheduler: self.scheduler,

            notifier: notifier.clone(),

            tree: Tree::default(),

            inheritance: InheritanceManager::default(),

            needs_build: Dirty::new(),

            callback_rx,
            callback_queue: CallbackQueue::new(callback_tx, notifier.clone()),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        };

        let root_id = manager.process_spawn(None, self.root.unwrap());

        manager.rebuild_queue.push_back(root_id);

        manager
    }
}
