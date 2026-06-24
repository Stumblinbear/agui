use std::any::Any;
use std::ptr::NonNull;

use crate::{
    context::{MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    render_object::node::RenderObjectPtr,
};

/// The type-erased, object-safe form of [`Element`], used at heterogeneous-children boundaries. Each method
/// forwards to the concrete element's matching lifecycle hook.
pub trait AnyElement {
    type Render: ?Sized;

    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn element_name(&self) -> &str;

    fn dyn_render_object_mut(&mut self) -> &mut Self::Render;

    fn dyn_render_object_ptr(&self) -> RenderObjectPtr<Self::Render>;

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx<'_>);

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx<'_>);

    fn dyn_rebuild(&mut self, ctx: &mut UpdateCtx<'_>);

    fn dyn_dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>);

    fn dyn_message(&mut self, ctx: &mut MessageCtx<'_>);

    fn dyn_describe(&self, d: &mut Diagnostics) -> DiagnosticsNode;
}

impl<T> AnyElement for T
where
    T: Any + Element,
{
    type Render = T::Render;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn element_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_render_object_mut(&mut self) -> &mut T::Render {
        self.render_object_mut()
    }

    fn dyn_render_object_ptr(&self) -> RenderObjectPtr<T::Render> {
        self.render_object_ptr()
    }

    fn dyn_mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.mount(ctx);
    }

    fn dyn_unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.unmount(ctx);
    }

    fn dyn_rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.rebuild(ctx);
    }

    fn dyn_dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.dependency_changed(ctx);
    }

    fn dyn_message(&mut self, ctx: &mut MessageCtx<'_>) {
        self.message(ctx);
    }

    fn dyn_describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.describe(d)
    }
}

/// Re-bases `ctx` onto the element behind `boxed`, so its inline children offset from the real element rather
/// than this box's pointer.
fn rebase_to_boxed<R: ?Sized>(
    boxed: &mut Box<dyn AnyElement<Render = R>>,
    ctx: &mut UpdateCtx<'_>,
) {
    // `addr_of_mut!` is a raw reborrow: it keeps the box's data-pointer provenance without a `&mut` tag that
    // would die before a child is dereferenced.
    let inner = std::ptr::addr_of_mut!(**boxed).cast::<()>();
    // SAFETY: `inner` is the boxed element's address, with provenance over its allocation.
    unsafe { ctx.rebase(NonNull::new_unchecked(inner)) };
}

impl<R: ?Sized> Element for Box<dyn AnyElement<Render = R>> {
    type Render = R;

    fn render_object_mut(&mut self) -> &mut R {
        (**self).dyn_render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        (**self).dyn_render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        rebase_to_boxed(self, ctx);
        (**self).dyn_mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        rebase_to_boxed(self, ctx);
        (**self).dyn_unmount(ctx);
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        rebase_to_boxed(self, ctx);
        (**self).dyn_rebuild(ctx);
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        rebase_to_boxed(self, ctx);
        (**self).dyn_dependency_changed(ctx);
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        (**self).dyn_message(ctx);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        (**self).dyn_describe(d)
    }
}
