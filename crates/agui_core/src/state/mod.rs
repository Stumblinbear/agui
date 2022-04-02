use core::panic;
use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use downcast_rs::{impl_downcast, Downcast};

mod listener;
pub(crate) mod map;

use fnv::FnvHashSet;
pub use listener::ListenerId;

use crate::engine::notify::Notifier;

pub trait Data: std::fmt::Debug + Downcast {}

impl<T> Data for T where T: std::fmt::Debug + 'static {}

impl_downcast!(Data);

enum StateRef<V> {
    Owned(Option<V>),
    Reference(Rc<V>),
}

impl<V> StateRef<V>
where
    V: Data + Clone,
{
    fn get(&self) -> &V {
        match &self {
            Self::Owned(value) => value.as_ref().expect("state missing"),
            Self::Reference(value) => value,
        }
    }
}

/// Holds the state of a value, with notify-on-write.
pub struct State<V>
where
    V: Data + Clone,
{
    notifier: Rc<Notifier>,
    listeners: Rc<RefCell<FnvHashSet<ListenerId>>>,

    value: StateRef<V>,

    updated_value: Rc<RefCell<Option<Rc<dyn Data>>>>,
}

impl<V> State<V>
where
    V: Data + Clone,
{
    pub(crate) fn new(
        notifier: Rc<Notifier>,
        listeners: Rc<RefCell<FnvHashSet<ListenerId>>>,
        value: Rc<V>,
        updated_value: Rc<RefCell<Option<Rc<dyn Data>>>>,
    ) -> Self {
        Self {
            notifier,
            listeners,

            value: StateRef::Reference(value),
            updated_value,
        }
    }

    /// Set the state.
    pub fn set(&mut self, value: V) {
        self.value = StateRef::Owned(Some(value));
    }
}

impl<V> Deref for State<V>
where
    V: Data + Clone,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.value.get()
    }
}

impl<V> DerefMut for State<V>
where
    V: Data + Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let StateRef::Reference(value) = &mut self.value {
            self.value = StateRef::Owned(Some(value.as_ref().clone()));
        }

        if let StateRef::Owned(value) = &mut self.value {
            value.as_mut().expect("state missing")
        } else {
            panic!("state value is not Owned");
        }
    }
}

impl<V> Drop for State<V>
where
    V: Data + Clone,
{
    fn drop(&mut self) {
        if let StateRef::Owned(value) = &mut self.value {
            let mut updated_value = self.updated_value.borrow_mut();

            if updated_value.is_some() {
                panic!("state values cannot be written twice in a single frame");
            }

            updated_value
                .replace(Rc::new(value.take().expect("state is gone")) as Rc<dyn Data>);

            self.notifier.notify_many(self.listeners.borrow().iter());
        }
    }
}

impl<V> std::fmt::Display for State<V>
where
    V: Data + Clone + std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.value.get(), f)
    }
}

impl<V> std::fmt::Debug for State<V>
where
    V: Data + Clone + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.value.get(), f)
    }
}
