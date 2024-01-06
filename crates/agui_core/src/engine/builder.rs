use std::collections::VecDeque;

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

pub struct EngineBuilder<P, N, const WITH_ROOT: bool = false> {
    plugins: P,

    notifier: N,

    root: Option<Widget>,
}

impl EngineBuilder<(), (), false> {
    pub(super) fn new() -> Self {
        Self {
            plugins: (),

            notifier: (),

            root: None,
        }
    }
}

impl<P, N, const WITH_ROOT: bool> EngineBuilder<P, N, WITH_ROOT>
where
    P: Plugin,
{
    pub fn add_plugin<T>(self, plugin: T) -> EngineBuilder<(T, P), N, WITH_ROOT>
    where
        T: Plugin,
    {
        EngineBuilder {
            plugins: (plugin, self.plugins),

            notifier: self.notifier,

            root: self.root,
        }
    }
}

impl<P, const WITH_ROOT: bool> EngineBuilder<P, (), WITH_ROOT> {
    pub fn with_notifier<N>(self, notifier: N) -> EngineBuilder<P, N, WITH_ROOT>
    where
        N: Fn() + 'static,
    {
        EngineBuilder {
            plugins: self.plugins,

            notifier,

            root: self.root,
        }
    }
}

impl<P, N> EngineBuilder<P, N, false> {
    pub fn with_root(self, root: impl IntoWidget) -> EngineBuilder<P, N, true> {
        EngineBuilder {
            plugins: self.plugins,

            notifier: self.notifier,

            root: Some(root.into_widget()),
        }
    }
}

impl<P, N> EngineBuilder<P, N, true>
where
    P: Plugin,
    N: Fn() + 'static,
{
    pub fn build(self) -> Engine {
        let mut engine = Engine {
            plugins: Plugins::new(self.plugins),

            bus: EventBus::default(),

            element_tree: Tree::default(),
            render_manager: RenderManager::default(),

            needs_build: Dirty::new(),
            callback_queue: CallbackQueue::new(self.notifier),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        };

        engine.init(self.root.expect("root is not set"));

        engine
    }
}

impl<P> EngineBuilder<P, (), true>
where
    P: Plugin,
{
    pub fn build(self) -> Engine {
        fn noop() {}

        let mut engine = Engine {
            plugins: Plugins::new(self.plugins),

            bus: EventBus::default(),

            element_tree: Tree::default(),
            render_manager: RenderManager::default(),

            needs_build: Dirty::new(),
            callback_queue: CallbackQueue::new(noop),

            rebuild_queue: VecDeque::default(),
            forgotten_elements: FxHashSet::default(),
        };

        engine.init(self.root.expect("root is not set"));

        engine
    }
}
