use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    engine::render::RenderManager,
    listenable::EventBus,
    plugin::{Plugin, Plugins},
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

use super::{Dirty, Engine};

pub struct EngineBuilder<P> {
    update_notifier_tx: Option<mpsc::Sender<()>>,

    root: Option<Widget>,

    plugins: P,
}

impl EngineBuilder<()> {
    pub(super) fn new() -> Self {
        Self {
            update_notifier_tx: None,

            root: None,

            plugins: (),
        }
    }
}

impl<P> EngineBuilder<P>
where
    P: Plugin,
{
    pub fn with_notifier(mut self, update_notifier_tx: mpsc::Sender<()>) -> Self {
        self.update_notifier_tx = Some(update_notifier_tx);
        self
    }

    pub fn with_root(mut self, root: impl IntoWidget) -> Self {
        self.root = Some(root.into_widget());
        self
    }

    pub fn add_plugin<T>(self, plugin: T) -> EngineBuilder<(T, P)>
    where
        T: Plugin,
    {
        EngineBuilder {
            update_notifier_tx: None,

            root: None,

            plugins: (plugin, self.plugins),
        }
    }

    pub fn build(self) -> Engine {
        let mut engine = Engine {
            plugins: Plugins::new(self.plugins),

            bus: EventBus::default(),

            element_tree: Tree::default(),
            render_manager: RenderManager::default(),

            needs_build: Dirty::new(),
            callback_queue: CallbackQueue::new(
                self.update_notifier_tx.unwrap_or_else(|| mpsc::channel().0),
            ),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        };

        engine.init(self.root.expect("root is not set"));

        engine
    }
}
