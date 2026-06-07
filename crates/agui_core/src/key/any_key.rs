use std::{
    any::Any,
    hash::{Hash, Hasher},
};

pub trait AnyKeyable: Any {
    fn as_any(&self) -> &dyn Any;

    fn dyn_eq(&self, other: &dyn Any) -> bool;

    fn dyn_hash(&self, state: &mut dyn Hasher);

    fn dyn_clone(&self) -> Box<dyn AnyKeyable>;
}

impl<T> AnyKeyable for T
where
    T: Hash + PartialEq + Eq + Any + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn Any) -> bool {
        if let Some(other) = other.downcast_ref::<T>() {
            self == other
        } else {
            false
        }
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        self.hash(&mut state);
    }

    fn dyn_clone(&self) -> Box<dyn AnyKeyable> {
        Box::new(self.clone())
    }
}
