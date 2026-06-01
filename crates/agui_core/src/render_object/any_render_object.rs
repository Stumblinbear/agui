use std::any::Any;

use crate::{context::UpdateCtx, render_object::RenderObject};

pub trait AnyRenderObject {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn render_object_name(&self) -> &str;

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx);

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx);
}

impl<T> AnyRenderObject for T
where
    T: Any,
    T: RenderObject,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render_object_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx) {
        self.mount(ctx);
    }

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx) {
        self.unmount(ctx);
    }
}

impl<T> RenderObject for Box<T>
where
    T: AnyRenderObject + ?Sized + 'static,
{
    fn mount(&mut self, ctx: &mut UpdateCtx) {
        (**self).dyn_mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        (**self).dyn_unmount(ctx);
    }
}
