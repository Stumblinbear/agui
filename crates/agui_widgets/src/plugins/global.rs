use std::{
    any::TypeId,
    cell::{Ref, RefCell},
    collections::HashSet,
    marker::PhantomData,
    rc::Rc,
};

use agui_core::{
    callback::CallbackContext,
    manager::{events::WidgetEvent, WidgetManager},
    plugin::{PluginContext, StatefulPlugin},
    unit::Data,
    util::map::{TypeMap, TypeSet, WidgetMap},
    widget::{ContextPlugins, ContextWidget, WidgetId, WidgetView},
};

#[derive(Debug, Default)]
pub struct GlobalPlugin;

impl StatefulPlugin for GlobalPlugin {
    type State = GlobalPluginState;

    // Check if any changes occurred outside of the main loop.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        for type_id in state.changed.drain() {
            let global = state.globals.get(&type_id).unwrap();

            for widget_id in &global.listeners {
                ctx.mark_dirty(*widget_id);
            }
        }
    }

    fn on_events(&self, _: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {
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

#[derive(Default)]
pub struct GlobalPluginState {
    globals: TypeMap<GlobalValue>,

    listening: WidgetMap<TypeSet>,

    changed: TypeSet,
}

pub struct GlobalValue {
    value: Rc<RefCell<dyn Data>>,

    listeners: HashSet<WidgetId>,
}

impl GlobalPluginState {
    fn get<G>(&mut self, widget_id: Option<WidgetId>) -> Global<G>
    where
        G: Data + Default,
    {
        let type_id = TypeId::of::<G>();

        let global = self.globals.entry(type_id).or_insert_with(|| {
            let value = G::default();

            tracing::debug!(
                id = widget_id
                    .map_or(String::from(""), |widget_id| format!("{:?}", widget_id))
                    .as_str(),
                r#type = format!("{:?}", std::any::type_name::<G>()).as_str(),
                // value = format!("{:?}", value).as_str(),
                "created new global"
            );

            GlobalValue {
                value: Rc::new(RefCell::new(value)),
                listeners: HashSet::default(),
            }
        });

        if let Some(widget_id) = widget_id {
            self.listening
                .entry(widget_id)
                .or_insert_with(TypeSet::default)
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
            listeners: HashSet::default(),
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

impl GlobalPluginExt for WidgetManager {
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default,
    {
        if let Some(plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().get(None)
        } else {
            tracing::warn!("GlobalPlugin not added");

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
        if let Some(plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().set(func)
        } else {
            tracing::warn!("GlobalPlugin not added");
        }
    }
}

pub trait ContextGlobalPluginExt {
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default;

    fn set_global<G, F>(&mut self, func: F)
    where
        F: FnOnce(&mut G) + 'static,
        G: Data + Default;
}

impl<C> ContextGlobalPluginExt for C
where
    C: ContextPlugins + ContextWidget,
{
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default,
    {
        let widget_id = self.get_widget_id();

        if let Some(plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().get(Some(widget_id))
        } else {
            tracing::warn!("GlobalPlugin not added");

            Global {
                phantom: PhantomData,

                value: Rc::new(RefCell::new(G::default())),
            }
        }
    }

    fn set_global<G, F>(&mut self, func: F)
    where
        F: FnOnce(&mut G) + 'static,
        G: Data + Default,
    {
        if let Some(plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().set(func)
        } else {
            tracing::warn!("GlobalPlugin not added")
        }
    }
}

impl<'ctx, W> GlobalPluginExt for CallbackContext<'ctx, W>
where
    W: WidgetView,
{
    fn get_global<G>(&mut self) -> Global<G>
    where
        G: Data + Default,
    {
        if let Some(plugin) = self.get_plugin_mut::<GlobalPlugin>() {
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
        if let Some(plugin) = self.get_plugin_mut::<GlobalPlugin>() {
            plugin.get_state_mut().set(func)
        }
    }
}

pub struct Global<G>
where
    G: Data,
{
    phantom: PhantomData<G>,

    value: Rc<RefCell<dyn Data>>,
}

impl<G> Global<G>
where
    G: Data,
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
    G: Data + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Global")
            .field("value", &self.borrow())
            .finish()
    }
}

impl<G> std::fmt::Display for Global<G>
where
    G: Data + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.borrow().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, cell::RefCell};

    use agui_core::{
        manager::WidgetManager,
        widget::{BuildContext, BuildResult, WidgetState, WidgetView},
    };
    use agui_macros::{StatefulWidget, StatelessWidget};
    use agui_primitives::Column;

    use super::{ContextGlobalPluginExt, GlobalPlugin, GlobalPluginExt};

    thread_local! {
        pub static STATE: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(Debug, Default, Clone, Copy)]
    struct TestGlobal(u32);

    #[derive(StatelessWidget, Clone, Debug, Default, PartialEq)]
    struct TestWidgetWriter {}

    impl WidgetView for TestWidgetWriter {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            ctx.set_global::<TestGlobal, _>(|value| value.0 += 1);

            BuildResult::empty()
        }
    }

    #[derive(StatefulWidget, Clone, Debug, Default, PartialEq)]
    struct TestWidgetReader {}

    impl WidgetState for TestWidgetReader {
        type State = u32;

        fn create_state(&self) -> Self::State {
            0
        }
    }

    impl WidgetView for TestWidgetReader {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            let global = ctx.get_global::<TestGlobal>();

            STATE.with(|f| {
                f.borrow_mut().push(global.borrow().0);
            });

            BuildResult::empty()
        }
    }

    #[test]
    pub fn tracks_listeners() {
        let mut manager = WidgetManager::with_root(TestWidgetReader::default());

        manager.add_plugin(GlobalPlugin::default());

        manager.update();

        let plugin = manager.get_plugin::<GlobalPlugin>().unwrap();
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
        let mut manager = WidgetManager::with_root(TestWidgetReader::default());

        manager.add_plugin(GlobalPlugin::default());

        manager.update();

        manager.set_root(TestWidgetReader::default());

        manager.update();

        let plugin = manager.get_plugin::<GlobalPlugin>().unwrap();
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
        let mut manager = WidgetManager::with_root(TestWidgetWriter::default());

        manager.add_plugin(GlobalPlugin::default());

        let global = manager.get_global::<TestGlobal>();

        assert_eq!(global.borrow().0, 0, "should init to default");

        manager.update();

        assert_eq!(global.borrow().0, 1, "should have updated to 1");
    }

    #[test]
    pub fn reading_globals() {
        let mut manager = WidgetManager::with_root(TestWidgetReader::default());

        manager.add_plugin(GlobalPlugin::default());

        let global = manager.get_global::<TestGlobal>();

        assert_eq!(global.borrow().0, 0, "should init to default");

        manager.set_global::<TestGlobal, _>(|value| {
            value.0 = 1;
        });

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[0], 1, "widget should have taken global value");
        });
    }

    #[test]
    pub fn reacting_to_globals() {
        let mut manager = WidgetManager::with_root(Column {
            children: [
                // Put the reader first so the writer will update the global
                TestWidgetReader::default().into(),
                TestWidgetWriter::default().into(),
            ]
            .into(),
            ..Default::default()
        });

        manager.add_plugin(GlobalPlugin::default());

        let global = manager.get_global::<TestGlobal>();

        assert_eq!(global.borrow().0, 0, "should init to default");

        manager.update();

        STATE.with(|f| {
            assert_eq!(
                f.borrow()[0],
                1,
                "widget should have taken global value after it was incremented"
            );
        });
    }
}
