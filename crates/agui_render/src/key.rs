use std::{any::Any, hash::Hash};

use bon::Builder;

pub use agui_core::key::{AnyKeyable, Keyable};

use crate::{
    context::{CreateCtx, UpdateCtx},
    render_object::box_layout::RenderBox,
    widget::Widget,
};

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

impl<V, Child> Widget for Key<V, Child>
where
    V: Clone + Hash + PartialEq + Eq + Any,
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = Child::Element;

    type Render = Child::Render;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        self.child.create(ctx)
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        self.child.update(ctx, element);
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        Some(&self.value)
    }
}
