use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{AnyElement, Element, RoutingId, node::ElementNode},
    key::AnyKeyable,
    render_object::AnyRenderObject,
    render_object::box_layout::{AnyRenderBox, RenderBox},
    render_object::sliver::{AnyRenderSliver, RenderSliver},
    widget::Widget,
};

/// The object-safe, type-erased form of [`Widget`].
pub trait AnyWidget {
    type Render;

    fn as_any(&self) -> &dyn Any;

    fn widget_name(&self) -> &str;

    fn dyn_widget_type_id(&self) -> TypeId;

    fn dyn_key(&self) -> Option<&dyn AnyKeyable>;

    fn dyn_create(self: Box<Self>, ctx: &mut UpdateCtx) -> (Box<dyn AnyElement>, Self::Render);

    fn dyn_update(
        self: Box<Self>,
        element: &mut Box<dyn AnyElement>,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    );
}

impl<T> AnyWidget for T
where
    T: Any + Widget,
    <T::Element as Element>::Render: AnyRenderObject + Sized,
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

    fn dyn_create(self: Box<Self>, ctx: &mut UpdateCtx) -> (Box<dyn AnyElement>, Self::Render) {
        let (element, render_object) = (*self).create(ctx);

        (Box::new(element), render_object)
    }

    fn dyn_update(
        self: Box<Self>,
        element: &mut Box<dyn AnyElement>,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        // The caller reconciles only same-type widgets, so the erased element is the one this
        // widget's type builds.
        let element = (**element)
            .as_any_mut()
            .downcast_mut::<T::Element>()
            .expect("element does not match its widget's type");

        (*self).update(element, render_object, ctx);
    }
}

/// The [`Element`] of a [`Box<dyn AnyWidget>`] erasure seam, holding the boxed child element and a
/// generation. A dispatch carries the generation as its leading [`RoutingId`]; one that no longer
/// matches addressed an inner that has since been replaced and is dropped.
pub struct ErasedElement<R> {
    generation: u16,

    type_id: TypeId,
    child: ElementNode<Box<dyn AnyElement>>,

    _render: PhantomData<fn() -> R>,
}

impl<R> Element for ErasedElement<R>
where
    R: AnyRenderObject + 'static,
{
    type Render = R;

    fn dispatch(&mut self, render: &mut R, path: &[RoutingId], action: Dispatch) {
        let Some((head, rest)) = path.split_first() else {
            // I'm not certain this is actually unreachable, but I can't prove it.
            unreachable!("the erasure seam pushes its generation as a leading routing id");
        };

        // A dispatch addressed to an older generation targeted an inner that has since been
        // replaced, so it is dropped rather than delivered to the replacement.
        if head.get() != self.generation {
            return;
        }

        let render: &mut dyn AnyRenderObject = render;
        self.child.element.dispatch(render, rest, action);
    }
}

impl<R> Widget for Box<dyn AnyWidget<Render = R>>
where
    R: AnyRenderObject + 'static,
{
    type Element = ErasedElement<R>;

    type Render = R;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let type_id = (*self).dyn_widget_type_id();

        let (child, render_object) =
            ctx.with_routing_id(RoutingId::new(0), |ctx| self.dyn_create(ctx));

        (
            ErasedElement {
                generation: 0,

                type_id,
                child: ElementNode::new(child),

                _render: PhantomData,
            },
            render_object,
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        let new_type = (*self).dyn_widget_type_id();

        if new_type == element.type_id {
            ctx.with_routing_id(RoutingId::new(element.generation), |ctx| {
                self.dyn_update(&mut element.child.element, render_object, ctx);
            });
        } else {
            // Type swap: bump the generation so events queued for the old inner are dropped, then
            // rebuild the inner element and its render object.
            element.generation = element.generation.wrapping_add(1);
            element.type_id = new_type;

            let (child, new_render) = ctx
                .with_routing_id(RoutingId::new(element.generation), |ctx| {
                    self.dyn_create(ctx)
                });

            element.child = ElementNode::new(child);
            *render_object = new_render;
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

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        (*self).create(ctx)
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        (*self).update(element, render_object, ctx);
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

struct RenderBoxWrapper<T> {
    inner: T,
}

impl<T> Widget for RenderBoxWrapper<T>
where
    T: Widget,
    T::Render: RenderBox,
{
    type Element = RenderBoxElement<T::Element>;

    type Render = Box<dyn AnyRenderBox>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (inner, render_object) = self.inner.create(ctx);

        (RenderBoxElement { inner }, Box::new(render_object))
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        // deref past the Box to the concrete object; same type -> reuse, else replace
        if let Some(render_object) = (**render_object).as_any_mut().downcast_mut::<T::Render>() {
            self.inner.update(&mut element.inner, render_object, ctx);
        } else {
            let (inner, render) = self.inner.create(ctx);
            element.inner = inner;
            *render_object = Box::new(render);
        }
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

struct RenderBoxElement<E> {
    inner: E,
}

impl<E> Element for RenderBoxElement<E>
where
    E: Element,
    E::Render: AnyRenderBox + Sized + 'static,
{
    type Render = Box<dyn AnyRenderBox>;

    fn dispatch(
        &mut self,
        render: &mut Box<dyn AnyRenderBox>,
        path: &[RoutingId],
        action: Dispatch,
    ) {
        let render = (**render)
            .as_any_mut()
            .downcast_mut::<E::Render>()
            .expect("a boxed render box keeps its inner render type for its whole life");

        self.inner.dispatch(render, path, action);
    }
}

struct RenderSliverWrapper<T> {
    inner: T,
}

impl<T> Widget for RenderSliverWrapper<T>
where
    T: Widget + 'static,
    T::Render: RenderSliver,
{
    type Element = RenderSliverElement<T::Element>;

    type Render = Box<dyn AnyRenderSliver>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (inner, render_object) = self.inner.create(ctx);

        (RenderSliverElement { inner }, Box::new(render_object))
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        if let Some(render_object) = (**render_object).as_any_mut().downcast_mut::<T::Render>() {
            self.inner.update(&mut element.inner, render_object, ctx);
        } else {
            let (inner, render) = self.inner.create(ctx);
            element.inner = inner;
            *render_object = Box::new(render);
        }
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

struct RenderSliverElement<E> {
    inner: E,
}

impl<E> Element for RenderSliverElement<E>
where
    E: Element,
    E::Render: AnyRenderSliver + Sized + 'static,
{
    type Render = Box<dyn AnyRenderSliver>;

    fn dispatch(
        &mut self,
        render: &mut Box<dyn AnyRenderSliver>,
        path: &[RoutingId],
        action: Dispatch,
    ) {
        let render = (**render)
            .as_any_mut()
            .downcast_mut::<E::Render>()
            .expect("a boxed render sliver keeps its inner render type for its whole life");

        self.inner.dispatch(render, path, action);
    }
}

pub type BoxedWidget = Box<dyn AnyWidget<Render = Box<dyn AnyRenderBox>>>;

pub type BoxedSliverWidget = Box<dyn AnyWidget<Render = Box<dyn AnyRenderSliver>>>;

pub trait AsAnyWidget: Widget + 'static {
    fn as_dyn_widget(&self) -> &dyn AnyWidget<Render = Self::Render>
    where
        Self: Sized,
        <Self::Element as Element>::Render: AnyRenderObject + Sized,
    {
        self
    }

    fn into_boxed_render_box(self) -> BoxedWidget
    where
        Self: Sized,
        Self::Render: RenderBox,
    {
        Box::new(RenderBoxWrapper { inner: self })
    }

    fn into_boxed_render_sliver(self) -> BoxedSliverWidget
    where
        Self: Sized,
        Self::Render: RenderSliver,
    {
        Box::new(RenderSliverWrapper { inner: self })
    }
}

impl<T: 'static> AsAnyWidget for T where T: Widget {}

#[cfg(test)]
mod tests {
    use std::{any::Any, cell::Cell, rc::Rc};

    use crate::{
        context::{Dispatch, MessageCtx, UpdateCtx},
        element::{Element, RoutingId},
        test_fixtures::Leaf,
        test_harness::with_ctx,
    };

    use super::{AnyRenderBox, AnyWidget, AsAnyWidget, ErasedElement, RenderBoxElement, Widget};

    struct TestWidget<T> {
        value: T,
        mounts: Rc<Cell<usize>>,
        updates: Rc<Cell<usize>>,
    }

    struct TestWidgetElement<T> {
        value: T,
    }

    impl<T: 'static> Element for TestWidgetElement<T> {
        type Render = ();
    }

    impl<T: 'static> Widget for TestWidget<T> {
        type Element = TestWidgetElement<T>;

        type Render = ();

        fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            self.mounts.set(self.mounts.get() + 1);

            (TestWidgetElement { value: self.value }, ())
        }

        fn update(self, element: &mut Self::Element, (): &mut Self::Render, _: &mut UpdateCtx) {
            self.updates.set(self.updates.get() + 1);

            element.value = self.value;
        }
    }

    /// A `TestWidget` plus the shared cells tracking its mount and update counts.
    fn test_widget<T>(value: T) -> (TestWidget<T>, Rc<Cell<usize>>, Rc<Cell<usize>>) {
        let mounts = Rc::new(Cell::new(0));
        let updates = Rc::new(Cell::new(0));

        let widget = TestWidget {
            value,
            mounts: Rc::clone(&mounts),
            updates: Rc::clone(&updates),
        };

        (widget, mounts, updates)
    }

    fn boxed_dyn<T: 'static>(widget: TestWidget<T>) -> Box<dyn AnyWidget<Render = ()>> {
        Box::new(widget)
    }

    /// The value held by the inner element behind a [`Box<dyn AnyWidget>`] boundary.
    fn dyn_value<T: Clone + 'static>(root: &ErasedElement<()>) -> T {
        (*root.child.element)
            .as_any()
            .downcast_ref::<TestWidgetElement<T>>()
            .expect("inner element type")
            .value
            .clone()
    }

    /// The value held by the inner element behind a boxed-render-box boundary.
    fn boxed_value<T: Clone + 'static>(root: &ErasedElement<Box<dyn AnyRenderBox>>) -> T {
        (*root.child.element)
            .as_any()
            .downcast_ref::<RenderBoxElement<TestWidgetElement<T>>>()
            .expect("inner element type")
            .inner
            .value
            .clone()
    }

    #[test]
    fn mounting_dyn_widgets() {
        let (widget, mounts, updates) = test_widget(7_usize);
        let (element, ()) = with_ctx(|ctx| boxed_dyn(widget).create(ctx));

        assert_eq!((mounts.get(), updates.get()), (1, 0));
        assert_eq!(dyn_value::<usize>(&element), 7);
    }

    #[test]
    fn mounting_boxed_widgets() {
        let (widget, _, _) = test_widget(1_usize);
        let (element, _) = with_ctx(|ctx| widget.into_boxed_render_box().create(ctx));

        assert_eq!(boxed_value::<usize>(&element), 1);
    }

    #[test]
    fn updating_dyn_widgets_reuses_the_inner() {
        let (widget, _, _) = test_widget(2_usize);
        let (mut element, mut render) = with_ctx(|ctx| boxed_dyn(widget).create(ctx));
        assert_eq!(dyn_value::<usize>(&element), 2);

        let (new_widget, new_mounts, new_updates) = test_widget(9_usize);
        with_ctx(|ctx| boxed_dyn(new_widget).update(&mut element, &mut render, ctx));

        // Same concrete type: the inner element is reconciled, not remounted.
        assert_eq!((new_mounts.get(), new_updates.get()), (0, 1));
        assert_eq!(dyn_value::<usize>(&element), 9);
    }

    #[test]
    fn updating_boxed_widgets() {
        let (widget, _, _) = test_widget(2_usize);
        let (mut element, mut render) = with_ctx(|ctx| widget.into_boxed_render_box().create(ctx));
        assert_eq!(boxed_value::<usize>(&element), 2);

        let (new_widget, _, _) = test_widget(9_usize);
        with_ctx(|ctx| {
            new_widget
                .into_boxed_render_box()
                .update(&mut element, &mut render, ctx);
        });

        assert_eq!(boxed_value::<usize>(&element), 9);
    }

    #[test]
    fn replacing_dyn_widgets_recreates_the_inner() {
        let (widget, _, _) = test_widget(2_usize);
        let (mut element, mut render) = with_ctx(|ctx| boxed_dyn(widget).create(ctx));

        let (new_widget, new_mounts, new_updates) = test_widget(7_u8);
        with_ctx(|ctx| boxed_dyn(new_widget).update(&mut element, &mut render, ctx));

        // Type changed, so the inner element is recreated, not updated.
        assert_eq!((new_mounts.get(), new_updates.get()), (1, 0));
        assert_eq!(dyn_value::<u8>(&element), 7);
    }

    #[test]
    fn replacing_boxed_widgets() {
        let (widget, _, _) = test_widget(2_usize);
        let (mut element, mut render) = with_ctx(|ctx| widget.into_boxed_render_box().create(ctx));
        assert_eq!(boxed_value::<usize>(&element), 2);

        let (new_widget, _, _) = test_widget(7_u8);
        with_ctx(|ctx| {
            new_widget
                .into_boxed_render_box()
                .update(&mut element, &mut render, ctx);
        });

        assert_eq!(boxed_value::<u8>(&element), 7);
    }

    fn leaf_widget(messages: &Rc<Cell<usize>>) -> BoxedDyn {
        let messages = Rc::clone(messages);
        Box::new(Leaf::new().on_message(move |_| messages.set(messages.get() + 1)))
    }

    type BoxedDyn = Box<dyn AnyWidget<Render = ()>>;

    #[test]
    fn dispatch_through_boundary_with_matching_generation_reaches_inner() {
        let messages = Rc::new(Cell::new(0_usize));
        let payload = Rc::new(Cell::new(None::<u32>));
        let widget: BoxedDyn = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            let payload = Rc::clone(&payload);
            move |ctx| {
                messages.set(messages.get() + 1);
                payload.set(Some(ctx.consume::<u32>()));
            }
        }));
        let (mut element, mut render) = with_ctx(|ctx| widget.create(ctx));

        // Initial generation is 0, so routing id 0 forwards to the inner.
        let mut msg = MessageCtx::new(Box::new(123_u32) as Box<dyn Any>);
        element.dispatch(
            &mut render,
            &[RoutingId::new(0)],
            Dispatch::Message(&mut msg),
        );

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(123));
    }

    #[test]
    fn dispatch_with_stale_generation_is_dropped() {
        let messages = Rc::new(Cell::new(0_usize));
        let (mut element, mut render) = with_ctx(|ctx| leaf_widget(&messages).create(ctx));

        // Generation is 0, so routing id 1 is stale and dropped.
        let mut msg = MessageCtx::new(Box::new(7_u32) as Box<dyn Any>);
        element.dispatch(
            &mut render,
            &[RoutingId::new(1)],
            Dispatch::Message(&mut msg),
        );

        assert_eq!(messages.get(), 0);
    }

    #[test]
    fn type_swap_bumps_generation_dropping_old_dispatches() {
        let messages = Rc::new(Cell::new(0_usize));
        let (mut element, mut render) = with_ctx(|ctx| leaf_widget(&messages).create(ctx));

        // Swap to a different concrete type, forcing a generation increment.
        let (other, _, _) = test_widget(0_u8);
        with_ctx(|ctx| boxed_dyn(other).update(&mut element, &mut render, ctx));

        // The old generation (0) is now stale, so the dispatch is dropped at the boundary.
        let mut msg = MessageCtx::new(Box::new(42_u32) as Box<dyn Any>);
        element.dispatch(
            &mut render,
            &[RoutingId::new(0)],
            Dispatch::Message(&mut msg),
        );

        assert_eq!(
            messages.get(),
            0,
            "the replaced inner must not receive messages addressed to the old generation"
        );
    }
}
