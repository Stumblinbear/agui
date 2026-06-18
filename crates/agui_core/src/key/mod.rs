use std::hash::{Hash, Hasher};

mod any_key;

pub use any_key::*;

pub trait Keyable {}

impl<T> Keyable for T where T: Hash + PartialEq + Eq {}

impl PartialEq for dyn AnyKeyable {
    fn eq(&self, other: &Self) -> bool {
        (*self).dyn_eq(other.as_any())
    }
}

impl Eq for dyn AnyKeyable {}

impl Hash for dyn AnyKeyable {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (*self).dyn_hash(state);
    }
}
