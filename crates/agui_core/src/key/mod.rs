use std::{
    any::Any,
    hash::{Hash, Hasher},
};

use bon::Builder;

use crate::{context::UpdateCtx, render_object::box_layout::RenderBox, widget::Widget};

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

impl<V, Child> Widget for Key<V, Child>
where
    V: Clone + Hash + PartialEq + Eq + Any,
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = Child::Element;

    type Render = Child::Render;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        self.child.create(ctx)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        self.child.update(element, render_object, ctx);
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        Some(&self.value)
    }
}
