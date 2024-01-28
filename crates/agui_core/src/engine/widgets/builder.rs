use std::{collections::VecDeque, sync::mpsc};

use agui_sync::notify;
use rustc_hash::FxHashSet;
use slotmap::SparseSecondaryMap;

use crate::{
    callback::CallbackQueue,
    engine::{
        widgets::bindings::{ElementBinding, ElementSchedulerBinding},
        widgets::{key_storage::WidgetKeyStorage, WidgetManager},
        Dirty,
    },
    inheritance::InheritanceManager,
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

pub struct WidgetManagerBuilder<EB, SB, const HAS_ROOT: bool> {
    element_binding: EB,
    scheduler: SB,

    notifier: Option<notify::Flag>,

    root: Option<Widget>,
}

impl Default for WidgetManagerBuilder<(), (), false> {
    fn default() -> Self {
        Self {
            element_binding: (),
            scheduler: (),

            notifier: None,

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

            notifier: self.notifier,

            root: self.root,
        }
    }
}

impl<EB, const HAS_ROOT: bool> WidgetManagerBuilder<EB, (), HAS_ROOT> {
    pub fn with_scheduler<SB>(self, scheduler: SB) -> WidgetManagerBuilder<EB, SB, HAS_ROOT>
    where
        SB: ElementSchedulerBinding,
    {
        WidgetManagerBuilder {
            element_binding: self.element_binding,
            scheduler,

            notifier: self.notifier,

            root: self.root,
        }
    }
}

impl<EB, SB, const HAS_ROOT: bool> WidgetManagerBuilder<EB, SB, HAS_ROOT> {
    pub fn with_notifier(self, notifier: notify::Flag) -> WidgetManagerBuilder<EB, SB, HAS_ROOT> {
        WidgetManagerBuilder {
            element_binding: self.element_binding,
            scheduler: self.scheduler,

            notifier: Some(notifier),

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

            notifier: self.notifier,

            root: Some(root.into_widget()),
        }
    }
}

impl<EB, SB> WidgetManagerBuilder<EB, SB, true>
where
    EB: ElementBinding,
    SB: ElementSchedulerBinding,
{
    pub fn build(self) -> WidgetManager<EB, SB> {
        let notifier = self.notifier.unwrap_or_default();

        let (callback_tx, callback_rx) = mpsc::channel();

        let mut manager = WidgetManager {
            element_binding: self.element_binding,
            scheduler: self.scheduler,

            tree: Tree::default(),
            deferred_resolvers: SparseSecondaryMap::default(),

            inheritance: InheritanceManager::default(),

            key_storage: WidgetKeyStorage::default(),

            needs_build: Dirty::new(notifier.clone()),

            callback_rx,
            callback_queue: CallbackQueue::new(callback_tx, notifier),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        };

        let root_id = manager.process_spawn(None, self.root.unwrap());

        manager.rebuild_queue.push_back(root_id);

        manager
    }
}
