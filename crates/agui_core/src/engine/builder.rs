use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    element::ElementId,
    listenable::EventBus,
    plugin::{Plugin, Plugins},
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

use super::{Dirty, Engine};

pub struct EngineBuilder<P> {
    root: Option<Widget>,

    plugins: P,
}

impl EngineBuilder<()> {
    pub(super) fn new() -> Self {
        Self {
            root: None,

            plugins: (),
        }
    }
}

impl<P> EngineBuilder<P>
where
    P: Plugin,
{
    pub fn with_root(mut self, root: impl IntoWidget) -> Self {
        self.root = Some(root.into_widget());
        self
    }

    pub fn add_plugin<T>(self, plugin: T) -> EngineBuilder<(T, P)>
    where
        T: Plugin,
    {
        EngineBuilder {
            root: None,

            plugins: (plugin, self.plugins),
        }
    }

    pub fn build(self) -> Engine {
        let (update_notifier_tx, update_notifier_rx) = mpsc::channel();

        let mut engine = Engine {
            plugins: Plugins::new(self.plugins),

            bus: EventBus::default(),

            update_notifier_rx,

            element_tree: Tree::default(),
            render_object_tree: Tree::default(),

            needs_build: Dirty::new(),
            callback_queue: CallbackQueue::new(update_notifier_tx),
            needs_layout: Dirty::new(),
            needs_paint: Dirty::new(),

            rebuild_queue: VecDeque::default(),
            removal_queue: FxHashSet::default(),

            sync_render_object_children: FxHashSet::default(),
            create_render_object: VecDeque::<ElementId>::default(),
            update_render_object: FxHashSet::default(),
        };

        engine.init(self.root.expect("root is not set"));

        engine
    }
}
