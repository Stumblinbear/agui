use std::collections::VecDeque;

use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    engine::{
        update_notifier::UpdateNotifier,
        widgets::{WidgetManager, WidgetManagerHooks},
        Dirty,
    },
    inheritance::InheritanceManager,
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

pub struct WidgetManagerBuilder<H, const HAS_ROOT: bool> {
    hooks: H,
    root: Option<Widget>,
}

impl Default for WidgetManagerBuilder<(), false> {
    fn default() -> Self {
        Self {
            hooks: (),
            root: None,
        }
    }
}

impl<const HAS_ROOT: bool> WidgetManagerBuilder<(), HAS_ROOT> {
    pub fn with_hooks<H>(self, hooks: H) -> WidgetManagerBuilder<H, HAS_ROOT>
    where
        H: WidgetManagerHooks,
    {
        WidgetManagerBuilder {
            hooks,
            root: self.root,
        }
    }
}

impl<H> WidgetManagerBuilder<H, false> {
    pub fn with_root<W>(self, root: W) -> WidgetManagerBuilder<H, true>
    where
        W: IntoWidget,
    {
        WidgetManagerBuilder {
            hooks: self.hooks,
            root: Some(root.into_widget()),
        }
    }
}

impl<H> WidgetManagerBuilder<H, true>
where
    H: WidgetManagerHooks,
{
    pub fn build(self) -> WidgetManager<H> {
        let notifier = UpdateNotifier::new();

        let mut manager = WidgetManager {
            hooks: self.hooks,

            notifier: notifier.clone(),

            tree: Tree::default(),

            inheritance: InheritanceManager::default(),

            needs_build: Dirty::new(),
            callback_queue: CallbackQueue::new(notifier.clone()),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        };

        let root_id = manager.process_spawn(None, self.root.unwrap());

        manager.rebuild_queue.push_back(root_id);

        manager
    }
}
