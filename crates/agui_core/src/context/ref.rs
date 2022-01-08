use std::{marker::PhantomData, sync::Arc};

use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

mod mouse;

pub use self::mouse::*;

pub trait Value: Downcast + Send + Sync + 'static {}

impl<T> Value for T where T: Send + Sync + 'static {}

impl_downcast!(Value);

pub struct State<V>
where
    V: Value,
{
    pub(crate) phantom: PhantomData<V>,

    pub(crate) on_changed: Option<Arc<Box<dyn Fn() + Send + Sync>>>,

    pub(crate) value: Arc<RwLock<Box<dyn Value>>>,
}

impl<V> State<V>
where
    V: Value,
{
    pub fn read(&self) -> MappedRwLockReadGuard<V> {
        RwLockReadGuard::map(
            self.value.read(),
            |value| match value.downcast_ref::<V>() {
                Some(value) => value,
                None => unreachable!(),
            },
        )
    }

    pub fn write(&self) -> MappedRwLockWriteGuard<V> {
        if let Some(func) = &self.on_changed {
            func();
        }

        RwLockWriteGuard::map(self.value.write(), |value| {
            match value.downcast_mut::<V>() {
                Some(value) => value,
                None => unreachable!(),
            }
        })
    }
}

pub struct ComputedRef<V>
where
    V: Value,
{
    pub(crate) phantom: PhantomData<V>,

    pub(crate) value: Arc<RwLock<Box<dyn Value>>>,
}

impl<V> ComputedRef<V>
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
}
