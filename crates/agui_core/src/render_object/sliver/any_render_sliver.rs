use std::any::Any;

use crate::render_object::{
    AnyRenderObject,
    sliver::{RenderSliver, SliverConstraints, SliverGeometry, SliverLayout},
};

pub trait AnyRenderSliver: AnyRenderObject {
    fn dyn_layout(&mut self, constraints: SliverConstraints) -> SliverGeometry;
}

impl<T> AnyRenderSliver for T
where
    T: Any,
    T: RenderSliver,
{
    fn dyn_layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
        SliverLayout::layout(self, constraints)
    }
}

impl<T> SliverLayout for Box<T>
where
    T: AnyRenderSliver + ?Sized + 'static,
{
    fn layout(&mut self, constraints: SliverConstraints) -> SliverGeometry {
        (**self).dyn_layout(constraints)
    }
}
