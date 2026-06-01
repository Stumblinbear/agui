use std::{any::Any, rc::Rc, sync::Arc};

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{Element, ElementState},
    key::AnyKeyable,
    render_object::{
        RenderObject,
        box_layout::{AnyRenderBox, RenderBox},
        sliver::{AnyRenderSliver, RenderSliver},
    },
    routing_id::RoutingId,
    view::View,
};

pub trait AnyView {
    type Render: RenderObject;

    fn as_any(&self) -> &dyn Any;

    fn view_name(&self) -> &str;

    fn dyn_mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState);

    fn dyn_update(
        &self,
        element: &mut Element,
        old: &dyn AnyView<Render = Self::Render>,
        ctx: &mut UpdateCtx,
    );

    fn dyn_dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch);

    fn dyn_create_render_object(&self, element: &Element) -> Self::Render;

    fn dyn_update_render_object(&self, element: &Element, render_object: &mut Self::Render);

    fn dyn_is_same_type(&self, other: &dyn AnyView<Render = Self::Render>) -> bool;

    fn dyn_key(&self) -> Option<&dyn AnyKeyable>;
}

impl<T, Render> AnyView for T
where
    T: Any,
    T: View<Render = Render>,
    Render: RenderObject,
{
    type Render = Render;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn view_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState) {
        let (children, state) = self.mount(ctx);

        (children, ElementState::new(state))
    }

    fn dyn_update(
        &self,
        element: &mut Element,
        old: &dyn AnyView<Render = Self::Render>,
        ctx: &mut UpdateCtx,
    ) {
        // Since we've erased the concrete View, this view will be not be re-mounted if the old view is not of the same type,
        // so we need to conditionally replace the element if the new view is not of the same type as the old view.
        if let Some(old) = old.as_any().downcast_ref::<Self>() {
            self.update(element, old, ctx);
        } else {
            *element = Element::new(self, ctx);
        }
    }

    fn dyn_dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        self.dispatch(element, path, action)
    }

    fn dyn_create_render_object(&self, element: &Element) -> Self::Render {
        self.create_render_object(element)
    }

    fn dyn_update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        self.update_render_object(element, render_object);
    }

    fn dyn_is_same_type(&self, other: &dyn AnyView<Render = Self::Render>) -> bool {
        other.as_any().is::<Self>()
    }

    fn dyn_key(&self) -> Option<&dyn AnyKeyable> {
        self.key()
    }
}

#[repr(transparent)]
pub struct AnyViewState {
    generation: u16,
}

macros::impl_view!(&dyn AnyView<Render = Render>);

macros::impl_view!(Box<dyn AnyView<Render = Render>>);

macros::impl_view!(Rc<dyn AnyView<Render = Render>>);

macros::impl_view!(Arc<dyn AnyView<Render = Render>>);

mod macros {
    // Used to implement View for the given smart pointer (e.g. Box, Rc, Arc)
    macro_rules! impl_view {
        (
            // The smart pointer type
            $ptr:ty
        ) => {
            impl<Render> View for $ptr
            where
                Render: RenderObject,
            {
                type Render = Render;

                type State = AnyViewState;

                fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
                    let (children, state) =
                        ctx.with_routing_id(RoutingId::new(0), |ctx| (**self).dyn_mount(ctx));

                    (
                        vec![Element { state, children }],
                        AnyViewState { generation: 0 },
                    )
                }

                fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
                    let state = element.state.downcast_mut::<Self>();

                    // If the type of the old view is not the same as the new view, we need to increment
                    // the generation. This is because events may have been queued up for the old view,
                    // and we don't want them to be erroneously sent to the new view. The generation
                    // is the routing id, so the new view will receive a new routing id, and thus won't
                    // receive the events that were queued up for the old view.
                    if !(**self).dyn_is_same_type(&**old) {
                        state.generation = state.generation.wrapping_add(1);
                    }

                    ctx.with_routing_id(RoutingId::new(state.generation), |ctx| {
                        (**self).dyn_update(&mut element.children[0], &**old, ctx)
                    });
                }

                fn dispatch(
                    &self,
                    element: &mut Element,
                    path: &[crate::routing_id::RoutingId],
                    action: crate::context::Dispatch,
                ) {
                    let generation = element.state.downcast_ref::<Self>().generation;

                    let Some((head, rest)) = path.split_first() else {
                        unreachable!("dispatch path cannot be empty");
                    };

                    // If the routing id is not the same as the generation, we don't want to send the message
                    // to the inner element since it has been replaced.
                    if head.get() != generation {
                        return;
                    }

                    (**self).dyn_dispatch(&mut element.children[0], rest, action)
                }

                fn create_render_object(&self, element: &Element) -> Self::Render {
                    (**self).dyn_create_render_object(&element.children[0])
                }

                fn update_render_object(
                    &self,
                    element: &Element,
                    render_object: &mut Self::Render,
                ) {
                    (**self).dyn_update_render_object(&element.children[0], render_object);
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

    pub(crate) use impl_view;
}

// The view-erasure adapter is protocol-specific: this one commits to the box layout protocol,
// and `SliverLayoutWrapper` (below) commits to the sliver protocol. They coexist because each
// targets a distinct erased render type (`Box<dyn AnyRenderBox>` vs `Box<dyn AnyRenderSliver>`),
// so there is no blanket-impl overlap to resolve.
struct BoxLayoutWrapper<T> {
    inner: T,
}

impl<T> View for BoxLayoutWrapper<T>
where
    T: View + 'static,
    T::Render: RenderBox,
{
    type Render = Box<dyn AnyRenderBox>;

    type State = T::State;

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        self.inner.mount(ctx)
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        self.inner.update(element, &old.inner, ctx);
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        self.inner.dispatch(element, path, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        Box::new(self.inner.create_render_object(element))
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        // deref past the Box to the concrete object; same type -> reuse, else replace
        if let Some(render_object) = (**render_object).as_any_mut().downcast_mut::<T::Render>() {
            self.inner.update_render_object(element, render_object);
        } else {
            *render_object = Box::new(self.inner.create_render_object(element));
        }
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.inner.is_same_type(&other.inner)
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

struct SliverLayoutWrapper<T> {
    inner: T,
}

impl<T> View for SliverLayoutWrapper<T>
where
    T: View + 'static,
    T::Render: RenderSliver,
{
    type Render = Box<dyn AnyRenderSliver>;

    type State = T::State;

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        self.inner.mount(ctx)
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        self.inner.update(element, &old.inner, ctx);
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        self.inner.dispatch(element, path, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        Box::new(self.inner.create_render_object(element))
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        if let Some(render_object) = (**render_object).as_any_mut().downcast_mut::<T::Render>() {
            self.inner.update_render_object(element, render_object);
        } else {
            *render_object = Box::new(self.inner.create_render_object(element));
        }
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.inner.is_same_type(&other.inner)
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

pub type BoxedView = Box<dyn AnyView<Render = Box<dyn AnyRenderBox>>>;

pub type BoxedSliverView = Box<dyn AnyView<Render = Box<dyn AnyRenderSliver>>>;

pub trait AsAnyView: View + 'static {
    fn as_dyn_view(&self) -> &dyn AnyView<Render = Self::Render>
    where
        Self: Sized,
    {
        self
    }

    fn into_boxed_render_box(self) -> BoxedView
    where
        Self: Sized,
        Self::Render: RenderBox,
    {
        Box::new(BoxLayoutWrapper { inner: self })
    }

    fn into_boxed_render_sliver(self) -> BoxedSliverView
    where
        Self: Sized,
        Self::Render: RenderSliver,
    {
        Box::new(SliverLayoutWrapper { inner: self })
    }
}

impl<T: 'static> AsAnyView for T where T: View {}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::{render_object::RenderLeaf, test_fixtures::Leaf, test_harness::TestHarness};

    use super::*;

    pub struct TestView<T> {
        value: T,
        mounts: Cell<usize>,
        updates: Cell<usize>,
    }

    impl<T> TestView<T> {
        pub fn new(value: T) -> Self {
            Self {
                value,
                mounts: Cell::new(0),
                updates: Cell::new(0),
            }
        }
    }

    impl<T> View for TestView<T>
    where
        T: Clone + 'static,
    {
        type Render = RenderLeaf;

        type State = T;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            self.mounts.set(self.mounts.get() + 1);

            (vec![], self.value.clone())
        }

        fn update(&self, element: &mut Element, _: &Self, _: &mut UpdateCtx) {
            self.updates.set(self.updates.get() + 1);

            *element.state.downcast_mut::<Self>() = self.value.clone();
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn mounting_dyn_views() {
        let view = TestView::new(7_usize);
        let harness = TestHarness::mount(&view.as_dyn_view());

        assert_eq!(view.mounts.get(), 1);
        assert_eq!(view.updates.get(), 0);
        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &7
        );
    }

    #[test]
    fn mounting_boxed_views() {
        let view = TestView::new(1_usize);
        let harness = TestHarness::mount(&view.into_boxed_render_box());

        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &1
        );
    }

    #[test]
    fn updating_dyn_views() {
        let view = TestView::new(2_usize);

        let mut harness = TestHarness::mount(&view.as_dyn_view());

        assert_eq!(view.mounts.get(), 1);
        assert_eq!(view.updates.get(), 0);
        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &2
        );

        let new_view = TestView::new(9_usize);
        harness.update(&view.as_dyn_view(), &new_view.as_dyn_view());

        assert_eq!(new_view.mounts.get(), 0);
        assert_eq!(new_view.updates.get(), 1);
        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &9
        );
    }

    #[test]
    fn updating_boxed_views() {
        let view = TestView::new(2_usize).into_boxed_render_box();

        let mut harness = TestHarness::mount(&view);

        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &2
        );

        let new_view = TestView::new(9_usize).into_boxed_render_box();
        harness.update(&view, &new_view);

        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &9
        );
    }

    #[test]
    fn replacing_dyn_views() {
        let view = TestView::new(2_usize);

        let mut harness = TestHarness::mount(&view.as_dyn_view());

        assert_eq!(view.mounts.get(), 1);
        assert_eq!(view.updates.get(), 0);
        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &2
        );

        let new_view = TestView::new(7_u8);
        harness.update(&view.as_dyn_view(), &new_view.as_dyn_view());

        // Type changed, so mount is called on the new view (not update)
        assert_eq!(new_view.mounts.get(), 1);
        assert_eq!(new_view.updates.get(), 0);
        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<u8>>(),
            &7
        );
    }

    #[test]
    fn replacing_boxed_views() {
        let view = TestView::new(2_usize).into_boxed_render_box();

        let mut harness = TestHarness::mount(&view);

        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<usize>>(),
            &2
        );

        harness.update(&view, &TestView::new(7_u8).into_boxed_render_box());

        assert_eq!(
            harness.root.children[0]
                .state
                .downcast_ref::<TestView<u8>>(),
            &7
        );
    }

    #[test]
    fn dispatch_message_through_boundary_with_matching_generation() {
        let messages = Rc::new(Cell::new(0_usize));
        let payload = Rc::new(Cell::new(None::<u32>));
        let view: Box<dyn AnyView<Render = RenderLeaf>> = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            let payload = Rc::clone(&payload);
            move |ctx| {
                messages.set(messages.get() + 1);
                payload.set(Some(ctx.consume::<u32>()));
            }
        }));
        let mut harness = TestHarness::mount(&view);

        // Initial generation is 0, so a routing id of 0 forwards to the inner view.
        let _ = harness.dispatch_message(&view, &[RoutingId::new(0)], Box::new(123_u32));

        assert_eq!(messages.get(), 1);
        assert_eq!(payload.get(), Some(123));
    }

    #[test]
    fn dispatch_rebuild_through_boundary_reaches_inner() {
        let rebuilds = Rc::new(Cell::new(0_usize));
        let messages = Rc::new(Cell::new(0_usize));
        let view: Box<dyn AnyView<Render = RenderLeaf>> = Box::new(
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
        let mut harness = TestHarness::mount(&view);

        harness.dispatch_rebuild(&view, &[RoutingId::new(0)]);

        assert_eq!(rebuilds.get(), 1);
        assert_eq!(messages.get(), 0);
    }

    #[test]
    fn dispatch_with_stale_generation_is_silently_dropped() {
        let messages = Rc::new(Cell::new(0_usize));
        let view: Box<dyn AnyView<Render = RenderLeaf>> = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            move |_| messages.set(messages.get() + 1)
        }));
        let mut harness = TestHarness::mount(&view);

        // Initial generation is 0, so a routing id of 1 is stale and should be dropped.
        let _ = harness.dispatch_message(&view, &[RoutingId::new(1)], Box::new(7_u32));

        assert_eq!(messages.get(), 0);
    }

    #[test]
    fn type_swap_increments_generation_dropping_old_dispatches() {
        let messages = Rc::new(Cell::new(0_usize));
        let view_a: Box<dyn AnyView<Render = RenderLeaf>> = Box::new(Leaf::new().on_message({
            let messages = Rc::clone(&messages);
            move |_| messages.set(messages.get() + 1)
        }));
        let mut harness = TestHarness::mount(&view_a);

        // Swap to a different concrete type, which forces a generation increment.
        let view_b: Box<dyn AnyView<Render = RenderLeaf>> = Box::new(TestView::new(0_u8));
        harness.update(&view_a, &view_b);

        // The old generation (0) is stale, so the dispatch should be dropped at the boundary
        // and never reach the replaced inner.
        let _ = harness.dispatch_message(&view_b, &[RoutingId::new(0)], Box::new(42_u32));

        assert_eq!(
            messages.get(),
            0,
            "the replaced inner must not receive messages addressed to the old generation"
        );
    }

    struct Counted {
        creates: Rc<Cell<usize>>,
    }

    impl View for Counted {
        type Render = RenderLeaf;
        type State = ();

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], ())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn create_render_object(&self, _: &Element) -> Self::Render {
            self.creates.set(self.creates.get() + 1);

            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn updating_boxed_view_reuses_render_object() {
        let creates = Rc::new(Cell::new(0usize));

        let view = Counted {
            creates: Rc::clone(&creates),
        }
        .into_boxed_render_box();

        let harness = TestHarness::mount(&view);

        let mut ro = view.create_render_object(&harness.root);
        assert_eq!(creates.get(), 1);

        view.update_render_object(&harness.root, &mut ro);

        assert_eq!(
            creates.get(),
            1,
            "same-type update must reuse the render object, not recreate it"
        );
    }
}
