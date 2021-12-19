use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{Mutex, RwLock, MappedRwLockReadGuard, RwLockReadGuard, RwLockWriteGuard, MappedRwLockWriteGuard};

use crate::widget::WidgetID;

use super::ListenerID;

type StateValue = Arc<RwLock<Box<dyn Value>>>;

pub struct StateMap {
    states: Mutex<HashMap<TypeId, StateValue>>,

    notify: Arc<Mutex<HashSet<ListenerID>>>,

    listener: Arc<dyn Fn() + Send + Sync>,
}

impl StateMap {
    pub fn new(on_changed: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>) -> StateMap {
        let notify = Arc::new(Mutex::new(HashSet::new()));

        StateMap {
            states: Default::default(),

            notify: notify.clone(),

            listener: {
                Arc::new(Box::new(move || {
                    on_changed(&notify.lock());
                }))
            },
        }
    }

    pub fn add_listener(&self, listener_id: ListenerID) {
        self.notify.lock().insert(listener_id);
    }

    pub fn remove_listener(&self, listener_id: &ListenerID) {
        self.notify.lock().remove(listener_id);
    }

    pub fn contains<V>(&self) -> bool
    where
        V: Value,
    {
        self.states.lock().contains_key(&TypeId::of::<V>())
    }

    pub fn insert<V>(&self, value: V)
    where
        V: Value,
    {
        self.states
            .lock()
            .insert(TypeId::of::<V>(), Arc::new(RwLock::new(Box::new(value))));
    }

    pub fn get<V>(&self) -> Option<Ref<V>>
    where
        V: Value,
    {
        if let Some(value) = self.states.lock().get(&TypeId::of::<V>()) {
            return Some(Ref {
                phantom: PhantomData,
                on_changed: self.listener.clone(),
                value: value.clone(),
            });
        }

        None
    }
}

pub struct WidgetStates {
    widgets: Mutex<HashMap<WidgetID, StateMap>>,

    listener: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>,
}

impl WidgetStates {
    pub fn new(listener: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>) -> WidgetStates {
        WidgetStates {
            widgets: Default::default(),
            listener,
        }
    }

    fn ensure_widget(&self, widget_id: &WidgetID) {
        let mut widgets = self.widgets.lock();

        if !widgets.contains_key(widget_id) {
            widgets.insert(*widget_id, StateMap::new(self.listener.clone()));
        }
    }

    pub fn set<V>(&self, listener_id: &ListenerID, value: V) -> Ref<V>
    where
        V: Value,
    {
        let widget_id = listener_id.widget_id();

        self.ensure_widget(widget_id);

        let widgets = self.widgets.lock();

        let state = widgets.get(widget_id).unwrap();

        state.insert(value);

        state.add_listener(*listener_id);

        state.get().expect("failed to get state")
    }

    pub fn get<V, F>(&self, listener_id: &ListenerID, func: F) -> Ref<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let widget_id = listener_id.widget_id();

        self.ensure_widget(widget_id);

        let widgets = self.widgets.lock();

        let state = widgets.get(widget_id).unwrap();

        if !state.contains::<V>() {
            state.insert(func());
        }

        state.add_listener(*listener_id);

        state.get().expect("failed to get state")
    }

    pub fn remove(&self, widget_id: &WidgetID) {
        self.widgets.lock().remove(widget_id);

        // Remove any listeners attached to any widget state
        self.widgets
            .lock()
            .iter()
            .for_each(|(_, states)| states.remove_listener(&ListenerID::Widget(*widget_id)));
    }
}

pub trait Value: Downcast + Send + Sync + 'static {}

impl<T> Value for T where T: Send + Sync + 'static {}

impl_downcast!(Value);

pub struct Ref<V>
where
    V: Value,
{
    pub(crate) phantom: PhantomData<V>,

    pub(crate) on_changed: Arc<dyn Fn() + Send + Sync>,

    pub(crate) value: Arc<RwLock<Box<dyn Value>>>,
}

impl<V> Ref<V>
where
    V: Value,
{
    pub fn read(&self) -> MappedRwLockReadGuard<V> {
        RwLockReadGuard::map(self.value.read(), |value| {
            value
                .downcast_ref::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }

    pub fn write(&self) -> MappedRwLockWriteGuard<V> {
        (self.on_changed)();

        RwLockWriteGuard::map(self.value.write(), |value| {
            value
                .downcast_mut::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }
}
