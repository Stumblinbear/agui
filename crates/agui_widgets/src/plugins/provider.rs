use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    collections::HashSet,
    marker::PhantomData,
    rc::Rc,
};

use agui_core::{
    manager::events::ElementEvent,
    plugin::{PluginContext, StatefulPlugin},
    unit::Data,
    util::map::{TypeMap, TypeSet, WidgetMap},
    widget::{BuildContext, ContextPlugins, ContextWidget, WidgetId, WidgetView},
};

#[derive(Debug, Default)]
pub struct ProviderPlugin;

impl StatefulPlugin for ProviderPlugin {
    type State = ProviderPluginState;

    // Check if any changes occurred outside of the main loop.
    fn on_before_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        self.on_update(ctx, state);
    }

    fn on_update(&self, ctx: &mut PluginContext, state: &mut Self::State) {
        for (owner_id, type_id) in state.changed.borrow_mut().drain() {
            let provided = state
                .providers
                .get(&owner_id)
                .and_then(|types| types.get(&type_id))
                .unwrap();

            for widget_id in &provided.listeners {
                ctx.mark_dirty(*widget_id);
            }
        }
    }

    fn on_events(&self, _: &mut PluginContext, state: &mut Self::State, events: &[ElementEvent]) {
        for event in events {
            if let ElementEvent::Destroyed { widget_id, .. } = event {
                // Remove the provided values and any listeners of it
                if let Some(types) = state.providers.remove(widget_id) {
                    for (type_id, provided) in types {
                        if let Some(providers) = state.provided.get_mut(&type_id) {
                            providers.remove(widget_id);
                        }

                        for listener_id in &provided.listeners {
                            if let Some(listening_to) = state.listening.get_mut(listener_id) {
                                listening_to.remove(widget_id);
                            }
                        }
                    }
                }

                // If the widget is listening to something, remove it from the respective listeners
                if let Some(providers) = state.listening.remove(widget_id) {
                    // Remove the widget from the listeners
                    for (owner_id, types) in providers {
                        for type_id in types {
                            if let Some(providing) = state.providers.get_mut(&owner_id) {
                                if let Some(provided) = providing.get_mut(&type_id) {
                                    provided.listeners.remove(widget_id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default)]
pub struct ProviderPluginState {
    providers: WidgetMap<TypeMap<ProvidedValue>>,
    provided: TypeMap<FnvHashSet<WidgetId>>,

    listening: WidgetMap<WidgetMap<TypeSet>>,

    changed: Rc<RefCell<HashSet<(WidgetId, TypeId)>>>,
}

pub struct ProvidedValue {
    value: Rc<RefCell<dyn Data>>,
    listeners: HashSet<WidgetId>,
}

impl ProviderPluginState {
    pub fn provide<V, F>(&mut self, widget_id: WidgetId, func: F) -> Provided<V>
    where
        V: Data,
        F: FnOnce() -> V,
    {
        let type_id = TypeId::of::<V>();

        let provided = self
            .providers
            .entry(widget_id)
            .or_insert_with(TypeMap::default)
            .entry(type_id)
            .or_insert_with(|| {
                let value = func();

                tracing::debug!(
                    id = format!("{:?}", widget_id).as_str(),
                    r#type = format!("{:?}", std::any::type_name::<V>()).as_str(),
                    // value = format!("{:?}", value).as_str(),
                    "provided new value"
                );

                ProvidedValue {
                    value: Rc::new(RefCell::new(value)),
                    listeners: HashSet::default(),
                }
            });

        self.provided
            .entry(type_id)
            .or_insert_with(FnvHashSet::default)
            .insert(widget_id);

        Provided {
            phantom: PhantomData,

            value: Rc::clone(&provided.value),

            owner: widget_id,
            changed: Rc::clone(&self.changed),
        }
    }

    pub fn consume<V>(&mut self, owner_id: WidgetId, widget_id: WidgetId) -> Option<Provided<V>>
    where
        V: Data,
    {
        let type_id = TypeId::of::<V>();

        let provided = self
            .providers
            .get_mut(&owner_id)
            .and_then(|values| values.get_mut(&type_id));

        if let Some(provided) = provided {
            provided.listeners.insert(widget_id);

            self.listening
                .entry(widget_id)
                .or_insert_with(WidgetMap::default)
                .entry(owner_id)
                .or_insert_with(TypeSet::default)
                .insert(type_id);

            Some(Provided {
                phantom: PhantomData,

                value: Rc::clone(&provided.value),

                owner: owner_id,
                changed: Rc::clone(&self.changed),
            })
        } else {
            None
        }
    }
}

pub trait ProviderPluginExt {
    fn provide<V, F>(&mut self, func: F) -> Provided<V>
    where
        V: Data,
        F: FnOnce() -> V;
}

impl<'ctx, W> ProviderPluginExt for BuildContext<'ctx, W>
where
    W: WidgetView,
{
    /// Makes some local widget state available to any child widget.
    fn provide<V, F>(&mut self, func: F) -> Provided<V>
    where
        V: Data,
        F: FnOnce() -> V,
    {
        let widget_id = self.get_widget_id();

        if let Some(plugin) = self.get_plugin_mut::<ProviderPlugin>() {
            plugin.get_state_mut().provide(widget_id, func)
        } else {
            Provided {
                phantom: PhantomData,

                owner: widget_id,

                value: Rc::new(RefCell::new(Box::new(func()))),

                changed: Rc::new(RefCell::new(HashSet::default())),
            }
        }
    }
}

pub trait ConsumerPluginExt {
    fn consume<V>(&mut self) -> Option<Provided<V>>
    where
        V: Data + Default;
}

impl<'ctx, W> ConsumerPluginExt for BuildContext<'ctx, W>
where
    W: WidgetView,
{
    /// Makes some local widget state available to any child widget.
    fn consume<V>(&mut self) -> Option<Provided<V>>
    where
        V: Data + Default,
    {
        let widget_id = self.get_widget_id();

        let mut owner_id = None;

        if let Some(plugin) = self.get_plugin::<ProviderPlugin>() {
            let tree = self.get_widgets();

            if let Some(providers) = plugin.get_state().provided.get(&TypeId::of::<V>()) {
                // If the widget calling this is also providing the state, return that.
                if providers.contains(&widget_id) {
                    owner_id = Some(widget_id);
                } else {
                    for parent_id in tree.iter_parents(widget_id) {
                        if providers.contains(&parent_id) {
                            owner_id = Some(parent_id);

                            break;
                        }
                    }
                }
            }
        }

        if let Some(owner_id) = owner_id {
            if let Some(plugin) = self.get_plugin_mut::<ProviderPlugin>() {
                plugin.get_state_mut().consume(owner_id, widget_id)
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub struct Provided<D>
where
    D: Data,
{
    phantom: PhantomData<D>,

    value: Rc<RefCell<dyn Data>>,

    owner: WidgetId,
    changed: Rc<RefCell<HashSet<(WidgetId, TypeId)>>>,
}

impl<D> Provided<D>
where
    D: Data,
{
    pub fn borrow(&self) -> Ref<D> {
        let borrowed = self.value.borrow();

        Ref::map(borrowed, |x| {
            x.downcast_ref::<D>()
                .expect("failed to downcast provided state")
        })
    }

    pub fn borrow_mut(&self) -> RefMut<D> {
        self.changed
            .borrow_mut()
            .insert((self.owner, TypeId::of::<D>()));

        let borrowed = self.value.borrow_mut();

        RefMut::map(borrowed, |x| {
            x.downcast_mut::<D>()
                .expect("failed to downcast provided state")
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{any::TypeId, cell::RefCell};

    use agui_core::{
        manager::WidgetManager,
        widget::{BuildContext, BuildResult, WidgetRef, WidgetState, WidgetView},
    };
    use agui_macros::{StatefulWidget, StatelessWidget};

    use crate::plugins::{
        global::{ContextGlobalPluginExt, GlobalPlugin, GlobalPluginExt},
        provider::ProviderPlugin,
    };

    use super::{ConsumerPluginExt, ProviderPluginExt};

    thread_local! {
        pub static STATE: RefCell<Vec<u32>> = RefCell::default();
    }

    #[derive(StatelessWidget, Default, PartialEq)]
    struct TestWidgetProvider {
        child: WidgetRef,
    }

    impl WidgetView for TestWidgetProvider {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            let global = ctx.get_global::<u32>();

            let provided = ctx.provide(u32::default);

            *provided.borrow_mut() = *global.borrow();

            (&self.child).into()
        }
    }

    #[derive(StatefulWidget, Clone, Debug, Default, PartialEq)]
    struct TestWidgetConsumer;

    impl WidgetState for TestWidgetConsumer {
        type State = u32;

        fn create_state(&self) -> Self::State {
            0
        }
    }

    impl WidgetView for TestWidgetConsumer {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            let consumed = ctx.consume::<u32>().expect("failed to consume");

            STATE.with(|f| {
                f.borrow_mut().push(*consumed.borrow());
            });

            BuildResult::empty()
        }
    }

    #[test]
    pub fn can_provide_a_value() {
        let mut manager = WidgetManager::with_root(TestWidgetProvider {
            child: TestWidgetConsumer.into(),
        });

        manager.add_plugin(ProviderPlugin::default());

        manager.update();

        let plugin = manager.get_plugin::<ProviderPlugin>().unwrap();
        let providers = &plugin.get_state().providers;
        let provided = &plugin.get_state().provided;
        let listeners = &plugin
            .get_state()
            .providers
            .values()
            .next()
            .unwrap()
            .get(&TypeId::of::<u32>())
            .unwrap()
            .listeners;

        assert!(!providers.is_empty(), "should have provided the value");
        assert!(!provided.is_empty(), "widget should be mapped to the type");
        assert!(!listeners.is_empty(), "should have tracked the listener");
    }

    #[test]
    pub fn does_not_leak_memory() {
        let mut manager = WidgetManager::with_root(TestWidgetProvider {
            child: TestWidgetConsumer.into(),
        });

        manager.add_plugin(ProviderPlugin::default());

        manager.update();

        manager.set_root(TestWidgetProvider::default());

        manager.update();

        let plugin = manager.get_plugin::<ProviderPlugin>().unwrap();
        let providers = &plugin.get_state().providers;
        let listeners = &plugin
            .get_state()
            .providers
            .values()
            .next()
            .unwrap()
            .get(&TypeId::of::<u32>())
            .unwrap()
            .listeners;

        assert_eq!(providers.len(), 1, "only one provider should be tracked");
        assert_eq!(listeners.len(), 0, "no listeners should be tracked");
    }

    #[test]
    pub fn reacts_to_changes() {
        let mut manager = WidgetManager::with_root(TestWidgetProvider {
            child: TestWidgetConsumer.into(),
        });

        manager.add_plugin(GlobalPlugin::default());
        manager.add_plugin(ProviderPlugin::default());

        manager.update();

        STATE.with(|f| {
            assert_eq!(f.borrow()[0], 0, "should have taken the default value");
        });

        manager.set_global::<u32, _>(|state| *state = 1);

        manager.update();

        STATE.with(|f| {
            assert_eq!(
                f.borrow()[1],
                1,
                "widget should have taken global value after rebuild"
            );
        });

        manager.set_global::<u32, _>(|state| *state = 7);

        manager.update();

        STATE.with(|f| {
            assert_eq!(
                f.borrow()[2],
                7,
                "widget should have taken the new global value after rebuild"
            );
        });
    }
}
