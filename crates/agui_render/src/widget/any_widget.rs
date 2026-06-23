use std::any::{Any, TypeId};
use std::marker::PhantomData;

use agui_core::tree::Slot;

use crate::{
    context::{CreateCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{AnyElement, Element},
    key::AnyKeyable,
    render_object::box_layout::RenderBox,
    render_object::sliver::RenderSliver,
    widget::Widget,
};

/// The object-safe, type-erased form of [`Widget`].
pub trait AnyWidget {
    type Render: ?Sized;

    fn as_any(&self) -> &dyn Any;

    fn widget_name(&self) -> &str;

    fn dyn_widget_type_id(&self) -> TypeId;

    fn dyn_key(&self) -> Option<&dyn AnyKeyable>;

    fn dyn_create(
        self: Box<Self>,
        ctx: &mut CreateCtx,
    ) -> Box<dyn AnyElement<Render = Self::Render>>;

    fn dyn_update(
        self: Box<Self>,
        ctx: &mut UpdateCtx<'_>,
        element: &mut Box<dyn AnyElement<Render = Self::Render>>,
    );
}

impl<T> AnyWidget for T
where
    T: Any + Widget,
    T::Element: 'static,
{
    type Render = T::Render;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_widget_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn dyn_key(&self) -> Option<&dyn AnyKeyable> {
        self.key()
    }

    fn dyn_create(self: Box<Self>, ctx: &mut CreateCtx) -> Box<dyn AnyElement<Render = T::Render>> {
        Box::new((*self).create(ctx))
    }

    fn dyn_update(
        self: Box<Self>,
        ctx: &mut UpdateCtx<'_>,
        element: &mut Box<dyn AnyElement<Render = T::Render>>,
    ) {
        // The caller reconciles only same-type widgets, so the erased element is the one this widget's
        // type builds.
        let element = (**element)
            .as_any_mut()
            .downcast_mut::<T::Element>()
            .expect("element does not match its widget's type");

        (*self).update(ctx, element);
    }
}

/// The [`Element`] of a [`Box<dyn AnyWidget>`] erasure seam, holding the boxed inner element. A type
/// hot-swap deregisters the old inner before registering its replacement, so the old [`NodeHandle`] dies
/// and a dispatch still addressed to it is dropped rather than delivered to the replacement.
///
/// [`NodeHandle`]: agui_core::tree::NodeHandle
pub struct ErasedElement<R: ?Sized> {
    type_id: TypeId,
    child: Slot<Box<dyn AnyElement<Render = R>>>,
    _render: PhantomData<fn() -> R>,
}

impl<R: ?Sized + 'static> Element for ErasedElement<R> {
    type Render = R;

    fn render_object(&self) -> &R {
        self.child.get().render_object()
    }

    fn render_object_mut(&mut self) -> &mut R {
        self.child.get_mut().render_object_mut()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.mount(&mut self.child) };
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.get().describe(d)
    }
}

impl<R: ?Sized + 'static> Widget for Box<dyn AnyWidget<Render = R>> {
    type Element = ErasedElement<R>;

    type Render = R;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let type_id = (*self).dyn_widget_type_id();
        let child = self.dyn_create(ctx);

        ErasedElement {
            type_id,
            child: Slot::new(child),
            _render: PhantomData,
        }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        let new_type = (*self).dyn_widget_type_id();

        if new_type == element.type_id {
            // SAFETY: `element.child` is the erased element's own slot.
            unsafe { ctx.with_child(&mut element.child, |inner, ctx| self.dyn_update(ctx, inner)) };
        } else {
            // Type swap: tear the old inner down (its handle dies, so events queued for it are dropped),
            // swap in the replacement, and remount it under a fresh handle. The replacement owns its render
            // object, so swapping the inner element swaps the render too.
            element.type_id = new_type;

            // SAFETY: `element.child` is the erased element's own slot.
            unsafe { ctx.unmount(&mut element.child) };
            let new_child = ctx.inflate(|ctx| self.dyn_create(ctx));
            *element.child.get_mut() = new_child;
            // SAFETY: `element.child` is the erased element's own slot.
            unsafe { ctx.mount(&mut element.child) };
        }
    }

    fn widget_type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        (**self).dyn_widget_type_id()
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        (**self).dyn_key()
    }
}

impl<W> Widget for Box<W>
where
    W: Widget,
{
    type Element = W::Element;

    type Render = W::Render;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        (*self).create(ctx)
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        (*self).update(ctx, element);
    }

    fn widget_type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        (**self).widget_type_id()
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        (**self).key()
    }
}

/// Adapts a concrete box-render widget into the protocol-erased boxed form, so a heterogeneous run can hold
/// widgets of different types behind one render protocol.
pub struct RenderBoxWrapper<T> {
    inner: T,
}

impl<T> RenderBoxWrapper<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T> Widget for RenderBoxWrapper<T>
where
    T: Widget,
    T::Render: RenderBox + Sized,
{
    type Element = RenderBoxElement<T::Element>;

    type Render = dyn RenderBox;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let element = self.inner.create(ctx);

        RenderBoxElement { inner: element }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        self.inner.update(ctx, &mut element.inner);
    }

    fn widget_type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        self.inner.widget_type_id()
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

/// The [`Element`] of a [`RenderBoxWrapper`]: the same node as its inner element, re-typed to the erased
/// box render. Every lifecycle hook forwards to the inner.
pub struct RenderBoxElement<E> {
    inner: E,
}

impl<E> Element for RenderBoxElement<E>
where
    E: Element,
    E::Render: RenderBox + Sized,
{
    type Render = dyn RenderBox;

    fn render_object(&self) -> &dyn RenderBox {
        self.inner.render_object()
    }

    fn render_object_mut(&mut self) -> &mut dyn RenderBox {
        self.inner.render_object_mut()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.unmount(ctx);
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.rebuild(ctx);
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.dependency_changed(ctx);
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        self.inner.message(ctx);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.inner.describe(d)
    }
}

/// Adapts a concrete sliver-render widget into the protocol-erased boxed form.
struct RenderSliverWrapper<T> {
    inner: T,
}

impl<T> Widget for RenderSliverWrapper<T>
where
    T: Widget,
    T::Render: RenderSliver + Sized,
{
    type Element = RenderSliverElement<T::Element>;

    type Render = dyn RenderSliver;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let element = self.inner.create(ctx);

        RenderSliverElement { inner: element }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        self.inner.update(ctx, &mut element.inner);
    }

    fn widget_type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        self.inner.widget_type_id()
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

/// The [`Element`] of a [`RenderSliverWrapper`]: the same node as its inner element, re-typed to the erased
/// sliver render. Every lifecycle hook forwards to the inner.
struct RenderSliverElement<E> {
    inner: E,
}

impl<E> Element for RenderSliverElement<E>
where
    E: Element,
    E::Render: RenderSliver + Sized,
{
    type Render = dyn RenderSliver;

    fn render_object(&self) -> &dyn RenderSliver {
        self.inner.render_object()
    }

    fn render_object_mut(&mut self) -> &mut dyn RenderSliver {
        self.inner.render_object_mut()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.unmount(ctx);
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.rebuild(ctx);
    }

    fn dependency_changed(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.inner.dependency_changed(ctx);
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        self.inner.message(ctx);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.inner.describe(d)
    }
}

pub trait AsAnyWidget: Widget + 'static {
    fn as_dyn_widget(&self) -> &dyn AnyWidget<Render = Self::Render>
    where
        Self: Sized,
        Self::Element: 'static,
    {
        self
    }

    fn into_boxed_render_box(self) -> Box<dyn AnyWidget<Render = dyn RenderBox>>
    where
        Self: Sized,
        Self::Render: RenderBox + Sized,
    {
        Box::new(RenderBoxWrapper { inner: self })
    }

    fn into_boxed_render_sliver(self) -> Box<dyn AnyWidget<Render = dyn RenderSliver>>
    where
        Self: Sized,
        Self::Render: RenderSliver + Sized,
    {
        Box::new(RenderSliverWrapper { inner: self })
    }
}

impl<T> AsAnyWidget for T
where
    T: Widget + 'static,
    T::Element: 'static,
{
}
