use std::{
    any::TypeId,
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

use agui_core::{
    engine::{event::WidgetEvent, Data, Engine},
    plugin::{EnginePlugin, PluginContext},
    prelude::BuildContext,
    widget::WidgetId,
};

#[derive(Debug, Default)]
pub struct GlobalPlugin;

impl EnginePlugin for GlobalPlugin {
    type State = GlobalState;

    // Check if any changes occurred outside of the main engine loop.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut GlobalState) {
        self.on_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut GlobalState) {
        for type_id in state.changed.drain() {
            let global = state.globals.get(&type_id).unwrap();

            for widget_id in &global.listeners {
                ctx.mark_dirty(*widget_id);
            }
        }
    }

    fn on_events(&self, _: &mut PluginContext, state: &mut GlobalState, events: &[WidgetEvent]) {
        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
                // If the widget is listening to something, remove it from the respective listeners
                if let Some(types) = state.listening.remove(widget_id) {
                    for type_id in types {
                        state
                            .globals
                            .get_mut(&type_id)
                            .unwrap()
                            .listeners
                            .remove(widget_id);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct GlobalState {
    globals: HashMap<TypeId, GlobalValue>,

    listening: HashMap<WidgetId, HashSet<TypeId>>,

    changed: HashSet<TypeId>,
}

pub struct GlobalValue {
    value: Rc<RefCell<dyn Data>>,
    listeners: HashSet<WidgetId>,
}

impl std::fmt::Debug for GlobalValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Global")
            .field("value", &self.value.borrow())
            .field("listeners", &self.listeners.len())
            .finish()
    }
}

impl GlobalState {
    fn get<G>(&mut self, listener: Option<WidgetId>) -> Global<G>
    where
        G: Data + Default,
    {
        let type_id = TypeId::of::<G>();

        let global = self.globals.entry(type_id).or_insert_with(|| GlobalValue {
            value: Rc::new(RefCell::new(G::default())),
            listeners: HashSet::new(),
        });

        if let Some(widget_id) = listener {
            self.listening
                .entry(widget_id)
                .or_insert_with(HashSet::new)
                .insert(type_id);

            global.listeners.insert(widget_id);
        }

        Global {
            phantom: PhantomData,

            value: Rc::clone(&global.value),
        }
    }

    fn set<G, F>(&mut self, func: F)
    where
        F: FnOnce(&mut G) + 'static,
        G: Data + Default,
    {
        let type_id = TypeId::of::<G>();

        let global = self.globals.entry(type_id).or_insert_with(|| GlobalValue {
            value: Rc::new(RefCell::new(G::default())),
            listeners: HashSet::new(),
        });

        self.changed.insert(TypeId::of::<G>());

        func(
            global
                .value
                .borrow_mut()
                .downcast_mut::<G>()
                .expect("failed to downcast global"),
        );
    }
}

pub trait GlobalPluginExt {
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default;

    fn set_global<G, F>(&mut self, func: F)
    where
        F: FnOnce(&mut G) + 'static,
        G: Data + Default;
}

impl GlobalPluginExt for Engine {
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default,
    {
        if let Some(mut plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().get(None)
        } else {
            Global {
                phantom: PhantomData,

                value: Rc::new(RefCell::new(Box::new(G::default()))),
            }
        }
    }

    fn set_global<G, F>(&mut self, func: F)
    where
        F: FnOnce(&mut G) + 'static,
        G: Data + Default,
    {
        if let Some(mut plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().set(func)
        }
    }
}

impl<'ctx, S> GlobalPluginExt for BuildContext<'ctx, S>
where
    S: Data,
{
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default,
    {
        let widget_id = self.get_widget_id();

        if let Some(mut plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().get(Some(widget_id))
        } else {
            Global {
                phantom: PhantomData,

                value: Rc::new(RefCell::new(Box::new(G::default()))),
            }
        }
    }

    fn set_global<G, F>(&mut self, func: F)
    where
        F: FnOnce(&mut G) + 'static,
        G: Data + Default,
    {
        if let Some(mut plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().set(func)
        }
    }
}

pub struct Global<G>
where
    G: Data + Default,
{
    phantom: PhantomData<G>,

    value: Rc<RefCell<dyn Data>>,
}

impl<G> Global<G>
where
    G: Data + Default,
{
    pub fn borrow(&self) -> Ref<G> {
        let borrowed = self.value.borrow();

        Ref::map(borrowed, |x| {
            x.downcast_ref::<G>().expect("failed to downcast global")
        })
    }
}

impl<G> std::fmt::Debug for Global<G>
where
    G: Data + Default,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.borrow().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use agui_core::{
        engine::{query::WidgetQueryExt, Engine},
        prelude::*,
    };
    use agui_primitives::Column;

    use super::{GlobalPlugin, GlobalPluginExt};

    #[derive(Debug, Default, Copy, Clone)]
    struct TestGlobal(u32);

    #[derive(Clone, Debug, Default)]
    struct TestWidgetWriter {}

    impl StatelessWidget for TestWidgetWriter {
        fn build(&self, ctx: &mut BuildContext<()>) -> BuildResult {
            ctx.set_global::<TestGlobal, _>(|value| value.0 += 1);

            BuildResult::None
        }
    }

    #[derive(Clone, Debug, Default)]
    struct TestWidgetReader {}

    impl StatefulWidget for TestWidgetReader {
        type State = u32;

        fn build(&self, ctx: &mut BuildContext<u32>) -> BuildResult {
            let global = ctx.get_global::<TestGlobal>();

            ctx.set_state(move |value| {
                *value += global.borrow().0;
            });

            BuildResult::None
        }
    }

    #[test]
    pub fn tracks_listeners() {
        let mut engine = Engine::with_root(TestWidgetReader::default());

        engine.add_plugin(GlobalPlugin::default());

        engine.update();

        let plugin = engine.get_plugin::<GlobalPlugin>().unwrap();
        let listening = &plugin.get_state().listening;
        let listeners = &plugin
            .get_state()
            .globals
            .get(&TypeId::of::<TestGlobal>())
            .unwrap()
            .listeners;

        assert!(!listening.is_empty(), "should have tracked the widget");
        assert!(!listeners.is_empty(), "should have tracked the listener");
    }

    #[test]
    pub fn does_not_leak_memory() {
        let mut engine = Engine::with_root(TestWidgetReader::default());

        engine.add_plugin(GlobalPlugin::default());

        engine.update();

        engine.set_root(TestWidgetReader::default().into());

        engine.update();

        let plugin = engine.get_plugin::<GlobalPlugin>().unwrap();
        let listening = &plugin.get_state().listening;
        let listeners = &plugin
            .get_state()
            .globals
            .get(&TypeId::of::<TestGlobal>())
            .unwrap()
            .listeners;

        assert!(!listening.is_empty(), "only one widget should be tracked");
        assert!(!listeners.is_empty(), "only one widget should be listening");
    }

    #[test]
    pub fn writing_globals() {
        let mut engine = Engine::with_root(TestWidgetWriter::default());

        engine.add_plugin(GlobalPlugin::default());

        let global = engine.get_global::<TestGlobal>();

        assert_eq!(global.borrow().0, 0, "should init to default");

        engine.update();

        assert_eq!(global.borrow().0, 1, "should have updated to 1");
    }

    #[test]
    pub fn reading_globals() {
        let mut engine = Engine::with_root(TestWidgetReader::default());

        engine.add_plugin(GlobalPlugin::default());

        let global = engine.get_global::<TestGlobal>();

        assert_eq!(global.borrow().0, 0, "should init to default");

        engine.set_global::<TestGlobal, _>(|value| {
            value.0 = 1;
        });

        engine.update();

        assert_eq!(
            *engine
                .query()
                .by_type::<TestWidgetReader>()
                .next()
                .unwrap()
                .get_state(),
            1,
            "widget should have taken global value"
        );
    }

    #[test]
    pub fn reacting_to_globals() {
        let mut engine = Engine::with_root(Column {
            children: vec![
                // Put the reader first so the writer will update the global
                TestWidgetReader::default().into(),
                TestWidgetWriter::default().into(),
            ],
            ..Default::default()
        });

        engine.add_plugin(GlobalPlugin::default());

        let global = engine.get_global::<TestGlobal>();

        assert_eq!(global.borrow().0, 0, "should init to default");

        engine.update();

        assert_eq!(
            *engine
                .query()
                .by_type::<TestWidgetReader>()
                .next()
                .unwrap()
                .get_state(),
            1,
            "widget should have taken global value after it was incremented"
        );
    }
}
