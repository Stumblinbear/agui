use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
};

use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

use crate::widget::WidgetId;

use super::{ListenerID, Value};

pub struct StateValue {
    value: Option<Arc<RwLock<Box<dyn Value>>>>,

    notify: Arc<Mutex<HashSet<ListenerID>>>,
}

pub struct StateMap {
    states: RwLock<HashMap<TypeId, StateValue>>,

    changed: Arc<Mutex<HashSet<ListenerID>>>,
}

impl StateMap {
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(changed: Arc<Mutex<HashSet<ListenerID>>>) -> Self {
        Self {
            states: RwLock::default(),
            changed,
        }
    }

    fn ensure_state<V>(&self)
    where
        V: Value,
    {
        let mut states = self.states.write();

        states.entry(TypeId::of::<V>()).or_insert_with(|| {
            let notify = Arc::new(Mutex::new(HashSet::new()));

            StateValue {
                value: None,

                notify: Arc::clone(&notify),
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

    pub fn get<V>(&self) -> Option<State<V>>
    where
        V: Value,
    {
        if let Some(state) = self.states.read().get(&TypeId::of::<V>()) {
            if let Some(value) = &state.value {
                return Some(State {
                    phantom: PhantomData,

                    notify: Arc::clone(&state.notify),
                    changed: Arc::clone(&self.changed),

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

    changed: Arc<Mutex<HashSet<ListenerID>>>,
}

impl WidgetStates {
    pub fn new(changed: Arc<Mutex<HashSet<ListenerID>>>) -> Self {
        Self {
            widgets: Mutex::default(),
            changed,
        }
    }

    pub fn init<V, F>(&self, listener_id: &ListenerID, func: F) -> State<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let widget_id = listener_id.widget_id();

        let mut widgets = self.widgets.lock();

        let state = widgets
            .entry(*widget_id)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.changed)));

        if !state.contains::<V>() {
            state.insert(func());
        }

        state.get().expect("failed to get state")
    }

    pub fn set<V>(&self, listener_id: &ListenerID, value: V) -> State<V>
    where
        V: Value,
    {
        let widget_id = listener_id.widget_id();

        let mut widgets = self.widgets.lock();

        let state = widgets
            .entry(*widget_id)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.changed)));

        state.insert(value);

        state.get().expect("failed to get state")
    }

    pub fn get<V, F>(&self, listener_id: &ListenerID, func: F) -> State<V>
    where
        V: Value,
        F: FnOnce() -> V,
    {
        let widget_id = listener_id.widget_id();

        let mut widgets = self.widgets.lock();

        let state = widgets
            .entry(*widget_id)
            .or_insert_with(|| StateMap::new(Arc::clone(&self.changed)));

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

/// Holds the state of a value, with notify-on-write.
pub struct State<V>
where
    V: Value,
{
    pub(crate) phantom: PhantomData<V>,

    pub(crate) notify: Arc<Mutex<HashSet<ListenerID>>>,
    pub(crate) changed: Arc<Mutex<HashSet<ListenerID>>>,

    pub(crate) value: Arc<RwLock<Box<dyn Value>>>,
}

impl<V> Clone for State<V>
where
    V: Value,
{
    fn clone(&self) -> Self {
        Self {
            phantom: self.phantom,

            notify: Arc::clone(&self.notify),
            changed: Arc::clone(&self.changed),

            value: Arc::clone(&self.value),
        }
    }
}

#[allow(clippy::missing_panics_doc)]
impl<V> State<V>
where
    V: Value,
{
    /// Read the state.
    pub fn read(&self) -> MappedRwLockReadGuard<V> {
        RwLockReadGuard::map(self.value.read(), |value| {
            value
                .downcast_ref::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }

    /// Write to the state.
    ///
    /// This will trigger an update of any components listening to the state. Use only if something legitimately changes.
    pub fn write(&self) -> MappedRwLockWriteGuard<V> {
        self.changed.lock().extend(self.notify.lock().iter());

        RwLockWriteGuard::map(self.value.write(), |value| {
            value
                .downcast_mut::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
        })
    }
}
