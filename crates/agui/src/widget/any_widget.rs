use std::any::{Any, TypeId};

use agui_core::tree::Slot;

use crate::{
    context::{CreateCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{AnyElement, Element},
    key::AnyKeyable,
    render_object::{
        RenderGraft, RenderObject, SingleChildRenderObject,
        box_layout::RenderBox,
        node::{RenderObjectCell, RenderObjectPtr},
        sliver::RenderSliver,
    },
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

/// The [`Element`] of a `Box<dyn AnyWidget<Render = dyn RenderBox>>` erasure seam, holding the boxed inner
/// element behind a [`RenderGraft`] anchor. A type hot-swap deregisters the old inner before registering its
/// replacement, so the old [`NodeHandle`] dies and a dispatch still addressed to it is dropped rather than
/// delivered to the replacement. The anchor stays put across the swap, so its boundary can be marked to lay
/// the new subtree out.
///
/// [`NodeHandle`]: agui_core::tree::NodeHandle
pub struct ErasedBoxElement {
    type_id: TypeId,
    child: Slot<Box<dyn AnyElement<Render = dyn RenderBox>>>,
    render: RenderObjectCell<RenderGraft<dyn RenderBox>>,
}

// SAFETY: manages its single inner element only through the cursor child operations, and resolves its render
// object (the graft anchor) from its own `RenderObjectCell`.
unsafe impl Element for ErasedBoxElement {
    type Render = RenderGraft<dyn RenderBox>;

    fn render_object_mut(&mut self) -> &mut Self::Render {
        self.render.get_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        let mounted = unsafe { ctx.mount(&mut self.child) };
        let render = self.render.get_mut();
        render.adopt_child(mounted);
        render.attach(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().detach(ctx);
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.get().describe(d)
    }
}

impl Widget for Box<dyn AnyWidget<Render = dyn RenderBox>> {
    type Element = ErasedBoxElement;

    type Render = RenderGraft<dyn RenderBox>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let type_id = (*self).dyn_widget_type_id();
        let child = self.dyn_create(ctx);

        ErasedBoxElement {
            type_id,
            child: Slot::new(child),
            render: RenderObjectCell::new(RenderGraft::new()),
        }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        let new_type = (*self).dyn_widget_type_id();

        if new_type == element.type_id {
            // SAFETY: `element.child` is the erased element's own slot.
            unsafe { ctx.with_child(&mut element.child, |inner, ctx| self.dyn_update(ctx, inner)) };
        } else {
            // Type swap: tear the old inner down (its handle dies, so events queued for it are dropped) and
            // mount the replacement. The anchor outlives the swap, so mark its boundary to lay out the new
            // subtree, which has never been laid out.
            element.type_id = new_type;

            let scope = element.render.get().layout_scope();
            ctx.mark_needs_layout(scope);

            // SAFETY: `element.child` is the erased element's own slot.
            unsafe { ctx.unmount(&mut element.child) };
            let new_child = ctx.inflate(|ctx| self.dyn_create(ctx));
            *element.child.get_mut() = new_child;
            // SAFETY: `element.child` is the erased element's own slot.
            let mounted = unsafe { ctx.mount(&mut element.child) };
            element.render.get_mut().adopt_child(mounted);
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

/// The [`Element`] of a `Box<dyn AnyWidget<Render = dyn RenderSliver>>` erasure seam. It forwards its render
/// object straight to the boxed inner element, with no graft anchor, because slivers are not yet laid out. When
/// they are, and a type swap must re-lay, it gains a sliver graft anchor like [`ErasedBoxElement`].
pub struct ErasedSliverElement {
    type_id: TypeId,
    child: Slot<Box<dyn AnyElement<Render = dyn RenderSliver>>>,
}

// SAFETY: forwards child management and render resolution to its single inner element, which upholds the
// contract.
unsafe impl Element for ErasedSliverElement {
    type Render = dyn RenderSliver;

    fn render_object_mut(&mut self) -> &mut dyn RenderSliver {
        self.child.get_mut().render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<dyn RenderSliver> {
        self.child.get().render_object_ptr()
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

impl Widget for Box<dyn AnyWidget<Render = dyn RenderSliver>> {
    type Element = ErasedSliverElement;

    type Render = dyn RenderSliver;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let type_id = (*self).dyn_widget_type_id();
        let child = self.dyn_create(ctx);

        ErasedSliverElement {
            type_id,
            child: Slot::new(child),
        }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        let new_type = (*self).dyn_widget_type_id();

        if new_type == element.type_id {
            // SAFETY: `element.child` is the erased element's own slot.
            unsafe { ctx.with_child(&mut element.child, |inner, ctx| self.dyn_update(ctx, inner)) };
        } else {
            // Type swap: tear the old inner down (its handle dies, so events queued for it are dropped) and
            // mount the replacement. No layout mark yet, since slivers are not laid out.
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

// SAFETY: forwards child management to its inner element; `render_object_ptr` is the inner's, erased to
// `dyn RenderBox`, valid for the element's mounted life.
unsafe impl<E> Element for RenderBoxElement<E>
where
    E: Element,
    E::Render: RenderBox + Sized,
{
    type Render = dyn RenderBox;

    fn render_object_mut(&mut self) -> &mut dyn RenderBox {
        self.inner.render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<dyn RenderBox> {
        self.inner.render_object_ptr().into_box()
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

// SAFETY: forwards child management to its inner element; `render_object_ptr` is the inner's, erased to
// `dyn RenderSliver`, valid for the element's mounted life.
unsafe impl<E> Element for RenderSliverElement<E>
where
    E: Element,
    E::Render: RenderSliver + Sized,
{
    type Render = dyn RenderSliver;

    fn render_object_mut(&mut self) -> &mut dyn RenderSliver {
        self.inner.render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<dyn RenderSliver> {
        self.inner.render_object_ptr().into_sliver()
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
