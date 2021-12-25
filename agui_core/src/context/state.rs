use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use crate::widget::WidgetId;

use super::ListenerID;

type StateValue = Arc<RwLock<Box<dyn Value>>>;

pub struct State {
    value: Option<StateValue>,

    notify: Arc<Mutex<HashSet<ListenerID>>>,

    on_changed: Arc<dyn Fn() + Send + Sync>,
}

pub struct StateMap {
    states: RwLock<HashMap<TypeId, State>>,

    on_changed: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>,
}

impl StateMap {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(on_changed: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>) -> Self {
        Self {
            states: RwLock::default(),

            on_changed: Arc::clone(&on_changed),
        }
    }

    fn ensure_state<V>(&self)
    where
        V: Value,
    {
        let mut states = self.states.write();

        states.entry(TypeId::of::<V>()).or_insert_with(|| {
            let notify = Arc::new(Mutex::new(HashSet::new()));

            let on_changed = Arc::clone(&self.on_changed);

            State {
                value: None,

                notify: Arc::clone(&notify),

                on_changed: Arc::new(Box::new(move || {
                    on_changed(&notify.lock());
                })),
            }
        });
    }

    pub fn contains<V>(&self) -> bool
    where
        V: Value,
    {
        self.states
            .read()
            .get(&TypeId::of::<V>())
            .map_or(false, |state| state.value.is_some())
    }

    pub fn insert<V>(&self, value: V)
    where
        V: Value,
    {
        self.ensure_state::<V>();

        let mut states = self.states.write();

        let state = states.get_mut(&TypeId::of::<V>()).unwrap();

        state.value = Some(Arc::new(RwLock::new(Box::new(value))));
    }

    pub fn get<V>(&self) -> Option<Ref<V>>
    where
        V: Value,
    {
        if let Some(state) = self.states.read().get(&TypeId::of::<V>()) {
            if let Some(value) = &state.value {
                return Some(Ref {
                    phantom: PhantomData,
                    on_changed: Arc::clone(&state.on_changed),
                    value: Arc::clone(value),
                });
            }
        }

        None
    }

    pub fn add_listener<V>(&self, listener_id: ListenerID)
    where
        V: Value,
    {
        self.ensure_state::<V>();

        let mut states = self.states.write();

        let state = states.get_mut(&TypeId::of::<V>()).unwrap();

        state.notify.lock().insert(listener_id);
    }

    pub fn remove_listener(&self, listener_id: &ListenerID) {
        for state in self.states.write().values() {
            state.notify.lock().remove(listener_id);
        }
    }
}

pub struct WidgetStates {
    widgets: Mutex<HashMap<WidgetId, StateMap>>,

    listener: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>,
}

impl WidgetStates {
    pub fn new(listener: Arc<dyn Fn(&HashSet<ListenerID>) + Send + Sync>) -> Self {
        Self {
            widgets: Mutex::default(),
            listener,
        }
    }

    pub fn set<V>(&self, listener_id: &ListenerID, value: V) -> Ref<V>
    where
        V: Value,
    {
        let widget_id = listener_id.widget_id();

        let mut widgets = self.widgets.lock();

        let state = widgets
            .entry(*widget_id)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.listener)));

        state.insert(value);

        state.get().expect("failed to get state")
    }

    pub fn get<V, F>(&self, listener_id: &ListenerID, func: F) -> Ref<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let widget_id = listener_id.widget_id();

        let mut widgets = self.widgets.lock();

        let state = widgets
            .entry(*widget_id)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.listener)));

        if !state.contains::<V>() {
            state.insert(func());
        }

        state.add_listener::<V>(*listener_id);

        state.get().expect("failed to get state")
    }

    pub fn remove(&self, widget_id: &WidgetId) {
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

impl<V> Clone for Ref<V>
where
    V: Value,
{
    fn clone(&self) -> Self {
        Self {
            phantom: self.phantom,
            on_changed: Arc::clone(&self.on_changed),
            value: Arc::clone(&self.value),
        }
    }
}

#[allow(clippy::missing_panics_doc)]
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
