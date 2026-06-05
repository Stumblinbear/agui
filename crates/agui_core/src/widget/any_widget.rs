use std::{any::Any, rc::Rc, sync::Arc};

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{AnyElement, Element, ElementNode},
    key::AnyKeyable,
    render_object::{
        RenderObject,
        box_layout::{AnyRenderBox, RenderBox},
        sliver::{AnyRenderSliver, RenderSliver},
    },
    routing_id::RoutingId,
    widget::Widget,
};

/// The object-safe, type-erased form of [`Widget`].
pub trait AnyWidget {
    type Render;

    fn as_any(&self) -> &dyn Any;

    fn widget_name(&self) -> &str;

    fn dyn_create_element(&self, ctx: &mut UpdateCtx) -> Box<dyn AnyElement>;

    fn dyn_update(
        &self,
        element: &mut Box<dyn AnyElement>,
        old: &dyn AnyWidget<Render = Self::Render>,
        ctx: &mut UpdateCtx,
    );

    fn dyn_dispatch(&self, element: &mut dyn AnyElement, path: &[RoutingId], action: Dispatch);

    fn dyn_create_render_object(&self, element: &dyn AnyElement) -> Self::Render;

    fn dyn_update_render_object(&self, element: &dyn AnyElement, render_object: &mut Self::Render);

    fn dyn_is_same_type(&self, other: &dyn AnyWidget<Render = Self::Render>) -> bool;

    fn dyn_key(&self) -> Option<&dyn AnyKeyable>;
}

impl<T> AnyWidget for T
where
    T: Any + Widget,
{
    type Render = T::Render;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_create_element(&self, ctx: &mut UpdateCtx) -> Box<dyn AnyElement> {
        Box::new(self.create_element(ctx))
    }

    fn dyn_update(
        &self,
        element: &mut Box<dyn AnyElement>,
        old: &dyn AnyWidget<Render = Self::Render>,
        ctx: &mut UpdateCtx,
    ) {
        // Same concrete type -> reconcile the recovered element in place; different type -> replace
        // it wholesale (the erased element would otherwise never be re-created).
        if let Some(old) = old.as_any().downcast_ref::<T>() {
            let element = (**element)
                .as_any_mut()
                .downcast_mut::<T::Element>()
                .expect("element does not match its widget's type");

            self.update(element, old, ctx);
        } else {
            *element = Box::new(self.create_element(ctx));
        }
    }

    fn dyn_dispatch(&self, element: &mut dyn AnyElement, path: &[RoutingId], action: Dispatch) {
        let element = element
            .as_any_mut()
            .downcast_mut::<T::Element>()
            .expect("element does not match its widget's type");

        self.dispatch(element, path, action);
    }

    fn dyn_create_render_object(&self, element: &dyn AnyElement) -> Self::Render {
        let element = element
            .as_any()
            .downcast_ref::<T::Element>()
            .expect("element does not match its widget's type");

        self.create_render_object(element)
    }

    fn dyn_update_render_object(&self, element: &dyn AnyElement, render_object: &mut Self::Render) {
        let element = element
            .as_any()
            .downcast_ref::<T::Element>()
            .expect("element does not match its widget's type");

        self.update_render_object(element, render_object);
    }

    fn dyn_is_same_type(&self, other: &dyn AnyWidget<Render = Self::Render>) -> bool {
        other.as_any().is::<T>()
    }

    fn dyn_key(&self) -> Option<&dyn AnyKeyable> {
        self.key()
    }
}

/// The [`Element`] of a `dyn AnyWidget` boundary.
pub struct ErasedElement<R: RenderObject> {
    generation: u16,
    child: ElementNode<Box<dyn AnyElement>>,
    _render: std::marker::PhantomData<R>,
}

impl<R: RenderObject> Element for ErasedElement<R> {}

macros::impl_widget!(&dyn AnyWidget<Render = Render>);

macros::impl_widget!(Box<dyn AnyWidget<Render = Render>>);

macros::impl_widget!(Rc<dyn AnyWidget<Render = Render>>);

macros::impl_widget!(Arc<dyn AnyWidget<Render = Render>>);

mod macros {
    // Used to implement Widget for the given smart pointer (e.g. Box, Rc, Arc)
    macro_rules! impl_widget {
        (
            // The smart pointer type
            $ptr:ty
        ) => {
            impl<Render> Widget for $ptr
            where
                Render: RenderObject,
            {
                type Element = ErasedElement<Render>;

                type Render = Render;

                fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
                    let child = ctx
                        .with_routing_id(RoutingId::new(0), |ctx| (**self).dyn_create_element(ctx));

                    ErasedElement {
                        generation: 0,
                        child: ElementNode::new(child),
                        _render: ::core::marker::PhantomData,
                    }
                }

                fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
                    // If the type of the old widget is not the same as the new widget, increment the
                    // generation. Events may have been queued for the old widget; the generation is
                    // the routing id, so the replaced inner won't receive the old widget's events.
                    if !(**self).dyn_is_same_type(&**old) {
                        element.generation = element.generation.wrapping_add(1);
                    }

                    ctx.with_routing_id(RoutingId::new(element.generation), |ctx| {
                        (**self).dyn_update(&mut element.child.element, &**old, ctx)
                    });
                }

                fn dispatch(
                    &self,
                    element: &mut Self::Element,
                    path: &[crate::routing_id::RoutingId],
                    action: crate::context::Dispatch,
                ) {
                    let Some((head, rest)) = path.split_first() else {
                        unreachable!("dispatch path cannot be empty");
                    };

                    // If the routing id is not the same as the generation, don't deliver to the
                    // inner element since it has been replaced.
                    if head.get() != element.generation {
                        return;
                    }

                    (**self).dyn_dispatch(&mut *element.child.element, rest, action)
                }

                fn create_render_object(&self, element: &Self::Element) -> Self::Render {
                    (**self).dyn_create_render_object(&*element.child.element)
                }

                fn update_render_object(
                    &self,
                    element: &Self::Element,
                    render_object: &mut Self::Render,
                ) {
                    (**self).dyn_update_render_object(&*element.child.element, render_object);
                }

                fn is_same_type(&self, other: &Self) -> bool {
                    (**self).dyn_is_same_type(&**other)
                }

                fn key(&self) -> Option<&dyn AnyKeyable> {
                    (**self).dyn_key()
                }
            }
        };
    }

    pub(crate) use impl_widget;
}

struct RenderBoxWrapper<T> {
    inner: T,
}

struct RenderBoxElement<E> {
    inner: E,
}

impl<E> Element for RenderBoxElement<E> where E: Element {}

impl<T> Widget for RenderBoxWrapper<T>
where
    T: Widget + 'static,
    T::Render: RenderBox,
{
    type Element = RenderBoxElement<T::Element>;

    type Render = Box<dyn AnyRenderBox>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        RenderBoxElement {
            inner: self.inner.create_element(ctx),
        }
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        self.inner.update(&mut element.inner, &old.inner, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        self.inner.dispatch(&mut element.inner, path, action);
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        Box::new(self.inner.create_render_object(&element.inner))
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        // deref past the Box to the concrete object; same type -> reuse, else replace
        if let Some(render_object) = (**render_object).as_any_mut().downcast_mut::<T::Render>() {
            self.inner
                .update_render_object(&element.inner, render_object);
        } else {
            *render_object = Box::new(self.inner.create_render_object(&element.inner));
        }
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.inner.is_same_type(&other.inner)
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

struct RenderSliverWrapper<T> {
    inner: T,
}

struct RenderSliverElement<E> {
    inner: E,
}

impl<E> Element for RenderSliverElement<E> where E: Element {}

impl<T> Widget for RenderSliverWrapper<T>
where
    T: Widget + 'static,
    T::Render: RenderSliver,
{
    type Element = RenderSliverElement<T::Element>;

    type Render = Box<dyn AnyRenderSliver>;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        RenderSliverElement {
            inner: self.inner.create_element(ctx),
        }
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        self.inner.update(&mut element.inner, &old.inner, ctx);
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        self.inner.dispatch(&mut element.inner, path, action);
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        Box::new(self.inner.create_render_object(&element.inner))
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        if let Some(render_object) = (**render_object).as_any_mut().downcast_mut::<T::Render>() {
            self.inner
                .update_render_object(&element.inner, render_object);
        } else {
            *render_object = Box::new(self.inner.create_render_object(&element.inner));
        }
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.inner.is_same_type(&other.inner)
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

pub type BoxedWidget = Box<dyn AnyWidget<Render = Box<dyn AnyRenderBox>>>;

pub type BoxedSliverWidget = Box<dyn AnyWidget<Render = Box<dyn AnyRenderSliver>>>;

pub trait AsAnyWidget: Widget + 'static {
    fn as_dyn_widget(&self) -> &dyn AnyWidget<Render = Self::Render>
    where
        Self: Sized,
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
    use std::{cell::Cell, rc::Rc};

    use crate::{element::Element, test_fixtures::Leaf, test_harness::TestHarness};

    use super::*;

    pub struct TestWidget<T> {
        value: T,
        mounts: Cell<usize>,
        updates: Cell<usize>,
    }

    impl<T> TestWidget<T> {
        pub fn new(value: T) -> Self {
            Self {
                value,
                mounts: Cell::new(0),
                updates: Cell::new(0),
            }
        }
    }

    pub struct TestWidgetElement<T> {
        value: T,
    }

    impl<T: 'static> Element for TestWidgetElement<T> {}

    impl<T> Widget for TestWidget<T>
    where
        T: Clone + 'static,
    {
        type Element = TestWidgetElement<T>;

        type Render = ();

        fn create_element(&self, _: &mut UpdateCtx) -> TestWidgetElement<T> {
            self.mounts.set(self.mounts.get() + 1);

            TestWidgetElement {
                value: self.value.clone(),
            }
        }

        fn update(&self, element: &mut TestWidgetElement<T>, _: &Self, _: &mut UpdateCtx) {
            self.updates.set(self.updates.get() + 1);

            element.value = self.value.clone();
        }

        fn create_render_object(&self, _: &TestWidgetElement<T>) -> Self::Render {}

        fn update_render_object(&self, _: &TestWidgetElement<T>, _: &mut Self::Render) {}
    }

    /// Reads the value held by the inner `TestWidgetElement` behind a dyn-widget boundary.
    fn dyn_value<T: Clone + 'static>(root: &ErasedElement<()>) -> T {
        (*root.child.element)
            .as_any()
            .downcast_ref::<TestWidgetElement<T>>()
            .expect("inner element type")
            .value
            .clone()
    }

    /// Reads the value held by the inner `TestWidgetElement` behind a boxed-render-box-widget boundary.
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
        let widget = TestWidget::new(7_usize);
        let harness = TestHarness::mount(&widget.as_dyn_widget());

        assert_eq!(widget.mounts.get(), 1);
        assert_eq!(widget.updates.get(), 0);
        assert_eq!(dyn_value::<usize>(&harness.root.element), 7);
    }

    #[test]
    fn mounting_boxed_widgets() {
        let widget = TestWidget::new(1_usize);
        let harness = TestHarness::mount(&widget.into_boxed_render_box());

        assert_eq!(boxed_value::<usize>(&harness.root.element), 1);
    }

    #[test]
    fn updating_dyn_widgets() {
        let widget = TestWidget::new(2_usize);

        let mut harness = TestHarness::mount(&widget.as_dyn_widget());

        assert_eq!(widget.mounts.get(), 1);
        assert_eq!(widget.updates.get(), 0);
        assert_eq!(dyn_value::<usize>(&harness.root.element), 2);

        let new_widget = TestWidget::new(9_usize);
        harness.update(&widget.as_dyn_widget(), &new_widget.as_dyn_widget());

        assert_eq!(new_widget.mounts.get(), 0);
        assert_eq!(new_widget.updates.get(), 1);
        assert_eq!(dyn_value::<usize>(&harness.root.element), 9);
    }

    #[test]
    fn updating_boxed_widgets() {
        let widget = TestWidget::new(2_usize).into_boxed_render_box();

        let mut harness = TestHarness::mount(&widget);

        assert_eq!(boxed_value::<usize>(&harness.root.element), 2);

        let new_widget = TestWidget::new(9_usize).into_boxed_render_box();
        harness.update(&widget, &new_widget);

        assert_eq!(boxed_value::<usize>(&harness.root.element), 9);
    }

    #[test]
    fn replacing_dyn_widgets() {
        let widget = TestWidget::new(2_usize);

        let mut harness = TestHarness::mount(&widget.as_dyn_widget());

        assert_eq!(widget.mounts.get(), 1);
        assert_eq!(widget.updates.get(), 0);
        assert_eq!(dyn_value::<usize>(&harness.root.element), 2);

        let new_widget = TestWidget::new(7_u8);
        harness.update(&widget.as_dyn_widget(), &new_widget.as_dyn_widget());

        // Type changed, so the inner element is recreated (create_element), not updated.
        assert_eq!(new_widget.mounts.get(), 1);
        assert_eq!(new_widget.updates.get(), 0);
        assert_eq!(dyn_value::<u8>(&harness.root.element), 7);
    }

    #[test]
    fn replacing_boxed_widgets() {
        let widget = TestWidget::new(2_usize).into_boxed_render_box();

        let mut harness = TestHarness::mount(&widget);

        assert_eq!(boxed_value::<usize>(&harness.root.element), 2);

        harness.update(&widget, &TestWidget::new(7_u8).into_boxed_render_box());

        assert_eq!(boxed_value::<u8>(&harness.root.element), 7);
    }

    #[test]
    fn dispatch_message_through_boundary_with_matching_generation() {
        let messages = Rc::new(Cell::new(0_usize));
        let payload = Rc::new(Cell::new(None::<u32>));
        let widget: Box<dyn AnyWidget<Render = ()>> = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            let payload = Rc::clone(&payload);
            move |ctx| {
                messages.set(messages.get() + 1);
                payload.set(Some(ctx.consume::<u32>()));
            }
        }));
        let mut harness = TestHarness::mount(&widget);

        // Initial generation is 0, so a routing id of 0 forwards to the inner widget.
        let _ = harness.dispatch_message(&widget, &[RoutingId::new(0)], Box::new(123_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(123));
    }

    #[test]
    fn dispatch_rebuild_through_boundary_reaches_inner() {
        let rebuilds = Rc::new(Cell::new(0_usize));
        let messages = Rc::new(Cell::new(0_usize));
        let widget: Box<dyn AnyWidget<Render = ()>> = Box::new(
            Leaf::new()
                .on_message({
                    let messages = Rc::clone(&messages);
                    move |_| messages.set(messages.get() + 1)
                })
                .on_rebuild({
                    let rebuilds = Rc::clone(&rebuilds);
                    move |_| rebuilds.set(rebuilds.get() + 1)
                }),
        );
        let mut harness = TestHarness::mount(&widget);

        harness.dispatch_rebuild(&widget, &[RoutingId::new(0)]);

        assert_eq!(rebuilds.get(), 1);
        assert_eq!(messages.get(), 0);
    }

    #[test]
    fn dispatch_with_stale_generation_is_silently_dropped() {
        let messages = Rc::new(Cell::new(0_usize));
        let widget: Box<dyn AnyWidget<Render = ()>> = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            move |_| messages.set(messages.get() + 1)
        }));
        let mut harness = TestHarness::mount(&widget);

        // Initial generation is 0, so a routing id of 1 is stale and should be dropped.
        let _ = harness.dispatch_message(&widget, &[RoutingId::new(1)], Box::new(7_u32));

        assert_eq!(messages.get(), 0);
    }

    #[test]
    fn type_swap_increments_generation_dropping_old_dispatches() {
        let messages = Rc::new(Cell::new(0_usize));
        let widget_a: Box<dyn AnyWidget<Render = ()>> = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            move |_| messages.set(messages.get() + 1)
        }));
        let mut harness = TestHarness::mount(&widget_a);

        // Swap to a different concrete type, which forces a generation increment.
        let widget_b: Box<dyn AnyWidget<Render = ()>> = Box::new(TestWidget::new(0_u8));
        harness.update(&widget_a, &widget_b);

        // The old generation (0) is stale, so the dispatch should be dropped at the boundary
        // and never reach the replaced inner.
        let _ = harness.dispatch_message(&widget_b, &[RoutingId::new(0)], Box::new(42_u32));

        assert_eq!(
            messages.get(),
            0,
            "the replaced inner must not receive messages addressed to the old generation"
        );
    }

    struct Counted {
        creates: Rc<Cell<usize>>,
    }

    struct CountedElement;

    impl Element for CountedElement {}

    impl Widget for Counted {
        type Element = CountedElement;

        type Render = ();

        fn create_element(&self, _: &mut UpdateCtx) -> CountedElement {
            CountedElement
        }

        fn update(&self, _: &mut CountedElement, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &CountedElement) -> Self::Render {
            self.creates.set(self.creates.get() + 1);
        }

        fn update_render_object(&self, _: &CountedElement, _: &mut Self::Render) {}
    }

    #[test]
    fn updating_boxed_widget_reuses_render_object() {
        let creates = Rc::new(Cell::new(0usize));

        let widget = Counted {
            creates: Rc::clone(&creates),
        }
        .into_boxed_render_box();

        let harness = TestHarness::mount(&widget);

        let mut ro = widget.create_render_object(&harness.root.element);
        assert_eq!(creates.get(), 1);

        widget.update_render_object(&harness.root.element, &mut ro);

        assert_eq!(
            creates.get(),
            1,
            "same-type update must reuse the render object, not recreate it"
        );
    }
}
