use std::{collections::VecDeque, sync::mpsc};

use rustc_hash::FxHashSet;

use crate::{
    callback::CallbackQueue,
    plugin::Plugin,
    render::manager::RenderViewManager,
    util::tree::Tree,
    widget::{IntoWidget, Widget},
};

use super::Engine;

pub struct EngineBuilder {
    update_notifier_tx: Option<mpsc::Sender<()>>,

    root: Option<Widget>,

    plugins: Vec<Box<dyn Plugin>>,
}

impl EngineBuilder {
    pub(super) fn new() -> Self {
        Self {
            update_notifier_tx: None,

            root: None,

            plugins: Vec::default(),
        }
    }

    pub fn with_notifier(mut self, update_notifier_tx: mpsc::Sender<()>) -> Self {
        self.update_notifier_tx = Some(update_notifier_tx);
        self
    }

    pub fn with_root(mut self, root: impl IntoWidget) -> Self {
        self.root = Some(root.into_widget());
        self
    }

    pub fn add_plugin(mut self, plugin: impl Plugin + 'static) -> Self {
        self.plugins.push(Box::new(plugin));
        self
    }

    pub fn build(self) -> Engine {
        let mut engine = Engine {
            plugins: self.plugins,

            element_tree: Tree::default(),
            render_view_manager: RenderViewManager::default(),

            dirty: FxHashSet::default(),
            callback_queue: CallbackQueue::new(
                self.update_notifier_tx.unwrap_or_else(|| mpsc::channel().0),
            ),

            rebuild_queue: VecDeque::default(),
            retained_elements: FxHashSet::default(),
            removal_queue: FxHashSet::default(),

            element_events: Vec::default(),
        };

        engine.init_root(self.root.expect("root is not set"));

        engine
    }
}
