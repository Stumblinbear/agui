use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    collections::HashSet,
    marker::PhantomData,
    rc::Rc,
};

use agui_core::{
    engine::{context::Context, event::WidgetEvent, widget::WidgetBuilder, Data},
    plugin::{EnginePlugin, PluginContext},
    util::map::{TypeMap, TypeSet, WidgetMap, WidgetSet},
    widget::{BuildContext, WidgetId},
};

#[derive(Debug, Default)]
pub struct ProviderPlugin;

impl EnginePlugin for ProviderPlugin {
    type State = ProviderPluginState;

    // Check if any changes occurred outside of the main engine loop.
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

    fn on_events(&self, _: &mut PluginContext, state: &mut Self::State, events: &[WidgetEvent]) {
        for event in events {
            if let WidgetEvent::Destroyed { widget_id, .. } = event {
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

#[derive(Debug, Default)]
pub struct ProviderPluginState {
    providers: WidgetMap<TypeMap<ProvidedValue>>,
    provided: TypeMap<WidgetSet>,

    listening: WidgetMap<WidgetMap<TypeSet>>,

    changed: Rc<RefCell<HashSet<(WidgetId, TypeId)>>>,
}

pub struct ProvidedValue {
    value: Rc<RefCell<dyn Data>>,
    listeners: HashSet<WidgetId>,
}

impl std::fmt::Debug for ProvidedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Provided")
            .field("value", &self.value.borrow())
            .field("listeners", &self.listeners.len())
            .finish()
    }
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
                    value = format!("{:?}", value).as_str(),
                    "provided new value"
                );

                ProvidedValue {
                    value: Rc::new(RefCell::new(value)),
                    listeners: HashSet::default(),
                }
            });

        self.provided
            .entry(type_id)
            .or_insert_with(WidgetSet::default)
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
    W: WidgetBuilder,
{
    /// Makes some local widget state available to any child widget.
    fn provide<V, F>(&mut self, func: F) -> Provided<V>
    where
        V: Data,
        F: FnOnce() -> V,
    {
        let widget_id = self.get_widget_id();

        if let Some(mut plugin) = self.get_plugin_mut::<ProviderPlugin>() {
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
    W: WidgetBuilder,
{
    /// Makes some local widget state available to any child widget.
    fn consume<V>(&mut self) -> Option<Provided<V>>
    where
        V: Data + Default,
    {
        let widget_id = self.get_widget_id();

        let mut owner_id = None;

        if let Some(plugin) = self.get_plugin::<ProviderPlugin>() {
            let tree = self.get_tree();

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
            if let Some(mut plugin) = self.get_plugin_mut::<ProviderPlugin>() {
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

impl<D> std::fmt::Debug for Provided<D>
where
    D: Data + Default,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.borrow().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;

    use agui_core::{
        engine::{context::Context, query::WidgetQueryExt, Engine},
        unit::Key,
        widget::{BuildContext, BuildResult, StatefulWidget, StatelessWidget, Widget},
    };

    use crate::plugins::{
        global::{GlobalPlugin, GlobalPluginExt},
        provider::ProviderPlugin,
    };

    use super::{ConsumerPluginExt, ProviderPluginExt};

    #[derive(Debug, Default, Clone, Copy)]
    struct TestGlobal(u32);

    #[derive(Clone, Debug, Default)]
    struct TestWidgetProvider {
        child: Widget,
    }

    impl StatelessWidget for TestWidgetProvider {
        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            let global = ctx.get_global::<u32>();

            let provided = ctx.provide(u32::default);

            *provided.borrow_mut() = *global.borrow();

            ctx.key(Key::single(), (&self.child).clone()).into()
        }
    }

    #[derive(Clone, Debug, Default)]
    struct TestWidgetConsumer;

    impl StatefulWidget for TestWidgetConsumer {
        type State = u32;

        fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
            let consumed = ctx.consume::<u32>().expect("failed to consume");

            ctx.set_state(move |value| {
                *value = *consumed.borrow();
            });

            BuildResult::None
        }
    }

    #[test]
    pub fn can_provide_a_value() {
        let mut engine = Engine::with_root(TestWidgetProvider {
            child: TestWidgetConsumer.into(),
        });

        engine.add_plugin(ProviderPlugin::default());

        engine.update();

        let plugin = engine.get_plugin::<ProviderPlugin>().unwrap();
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
        let mut engine = Engine::with_root(TestWidgetProvider {
            child: TestWidgetConsumer.into(),
        });

        engine.add_plugin(ProviderPlugin::default());

        engine.update();

        engine.set_root(TestWidgetProvider::default().into());

        engine.update();

        let plugin = engine.get_plugin::<ProviderPlugin>().unwrap();
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
        let mut engine = Engine::with_root(TestWidgetProvider {
            child: TestWidgetConsumer.into(),
        });

        engine.add_plugin(GlobalPlugin::default());
        engine.add_plugin(ProviderPlugin::default());

        engine.update();

        assert_eq!(
            *engine
                .query()
                .by_type::<TestWidgetConsumer>()
                .next()
                .unwrap()
                .get_state(),
            0,
            "consumer should have taken the value provided"
        );

        engine.set_global::<u32, _>(|state| *state = 1);

        engine.update();

        assert_eq!(
            *engine
                .query()
                .by_type::<TestWidgetConsumer>()
                .next()
                .unwrap()
                .get_state(),
            1,
            "widget should have taken global value after rebuild"
        );

        engine.set_global::<u32, _>(|state| *state = 7);

        engine.update();

        assert_eq!(
            *engine
                .query()
                .by_type::<TestWidgetConsumer>()
                .next()
                .unwrap()
                .get_state(),
            7,
            "widget should have taken the new global value after rebuild"
        );
    }
}
