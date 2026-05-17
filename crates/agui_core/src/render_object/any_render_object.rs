use std::any::Any;

use crate::{
    context::UpdateCtx,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::RenderObject,
    renderer::Canvas,
};

pub trait AnyRenderObject {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn render_object_name(&self) -> &str;

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx);

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx);

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest;

    fn dyn_draw(&mut self, canvas: &mut Canvas);
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

    fn dyn_hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.hit_test(result, position)
    }

    fn dyn_draw(&mut self, canvas: &mut Canvas) {
        self.draw(canvas);
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

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        (**self).dyn_hit_test(result, position)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        (**self).dyn_draw(canvas);
    }
}

pub trait AsAnyRenderObject {
    type Output: AnyRenderObject + ?Sized;

    fn as_dyn_render_object(&self) -> &dyn AnyRenderObject;

    fn into_boxed_render_object(self) -> Box<Self::Output>;
}
