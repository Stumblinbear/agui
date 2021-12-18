use std::{sync::Arc, marker::PhantomData};

use downcast_rs::{impl_downcast, Downcast};

mod mouse;

pub use mouse::*;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard, MappedRwLockWriteGuard, MappedRwLockReadGuard};

pub trait Value: Downcast + Send + Sync + 'static {}

impl<T> Value for T where T: Send + Sync + 'static {}

impl_downcast!(Value);

pub struct Ref<V>
where
    V: Value,
{
    pub(crate) phantom: PhantomData<V>,
    
    pub(crate) on_changed: Option<Arc<Box<dyn Fn() + Send + Sync>>>,

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
        if let Some(func) = &self.on_changed {
            func();
        }

        RwLockWriteGuard::map(self.value.write(), |value| {
            value
                .downcast_mut::<V>()
                .unwrap_or_else(|| panic!("downcasting state failed"))
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