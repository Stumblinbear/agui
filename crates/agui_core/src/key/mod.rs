use std::{
    any::Any,
    hash::{Hash, Hasher},
};

use bon::Builder;

use crate::{
    context::{Dispatch, UpdateCtx},
    element::Element,
    routing_id::RoutingId,
    view::View,
};

mod any_key;

pub use any_key::*;

pub trait Keyable {}

impl<T> Keyable for T where T: Hash + PartialEq + Eq {}

#[derive(Builder, Debug)]
#[builder(start_fn = value)]
#[builder(finish_fn = child)]
pub struct Key<V, Child> {
    #[builder(start_fn)]
    value: V,

    #[builder(finish_fn)]
    child: Child,
}

impl<V, Child> Key<V, Child> {
    pub fn new(value: V, child: Child) -> Self {
        Self { value, child }
    }
}

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

impl<V, Child> View for Key<V, Child>
where
    V: Clone + Hash + PartialEq + Eq + Any,
    Child: View,
{
    type Render = Child::Render;

    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State)
    where
        Self: Sized,
    {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child_mut(0, &old.child).update(&self.child, ctx);
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        element.child_mut(0, &self.child).dispatch(path, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        self.child.create_render_object(element)
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        self.child.update_render_object(element, render_object)
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        Some(&self.value)
    }
}
