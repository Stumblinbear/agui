use std::{any::Any, rc::Rc, sync::Arc};

use crate::{
    context::{MessageCtx, UpdateCtx},
    element::{Element, ElementState},
    key::AnyKeyable,
    render_object::{AnyRenderObject, AsAnyRenderObject, RenderObject},
    view::{MountView, View},
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

    fn dyn_message(&self, element: &mut Element, ctx: MessageCtx);

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
        MountView::mount(self, ctx)
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

    fn dyn_message(&self, element: &mut Element, ctx: MessageCtx) {
        self.message(element, ctx);
    }

    fn dyn_create_render_object(&self, element: &Element) -> Self::Render {
        self.create_render_object(element)
    }

    fn dyn_update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        self.update_render_object(element, render_object);
    }

    fn dyn_is_same_type(&self, other: &dyn AnyView<Render = Self::Render>) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Self>() {
            self.is_same_type(other)
        } else {
            false
        }
    }

    fn dyn_key(&self) -> Option<&dyn AnyKeyable> {
        self.key()
    }
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

                type State = ElementState;

                fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
                    (**self).dyn_mount(ctx)
                }

                fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
                    (**self).dyn_update(element, &**old, ctx);
                }

                fn message(&self, element: &mut Element, ctx: MessageCtx) {
                    (**self).dyn_message(element, ctx);
                }

                fn create_render_object(&self, element: &Element) -> Self::Render {
                    (**self).dyn_create_render_object(element)
                }

                fn update_render_object(
                    &self,
                    element: &Element,
                    render_object: &mut Self::Render,
                ) {
                    (**self).dyn_update_render_object(element, render_object);
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

struct AnyViewWrapper<T> {
    inner: T,
}

impl<T> View for AnyViewWrapper<T>
where
    T: View + 'static,
    T::Render: Any,
    T::Render: AsAnyRenderObject,
{
    type Render = Box<<T::Render as AsAnyRenderObject>::Output>;

    type State = T::State;

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        self.inner.mount(ctx)
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        self.inner.update(element, &old.inner, ctx);
    }

    fn message(&self, element: &mut Element, ctx: MessageCtx) {
        self.inner.message(element, ctx);
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        self.inner
            .dyn_create_render_object(element)
            .into_boxed_render_object()
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        if let Some(render_object) = render_object.as_any_mut().downcast_mut::<T::Render>() {
            self.inner.update_render_object(element, render_object);
        } else {
            *render_object = self
                .inner
                .dyn_create_render_object(element)
                .into_boxed_render_object();
        }
    }

    fn is_same_type(&self, other: &Self) -> bool {
        self.inner.is_same_type(&other.inner)
    }

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.inner.key()
    }
}

#[allow(type_alias_bounds)]
pub type BoxedView<V: View> =
    Box<dyn AnyView<Render = Box<<V::Render as AsAnyRenderObject>::Output>>>;

pub trait AsAnyView: View + 'static {
    fn as_dyn_view(&self) -> &(dyn AnyView<Render = Self::Render>)
    where
        Self: Sized,
    {
        self
    }

    fn into_boxed_view(
        self,
    ) -> Box<dyn AnyView<Render = Box<<Self::Render as AsAnyRenderObject>::Output>>>
    where
        Self: Sized,
        Self::Render: AsAnyRenderObject,
    {
        Box::new(AnyViewWrapper { inner: self })
    }
}

impl<T: 'static> AsAnyView for T where T: View {}

// pub trait BoxedViewExt {
//     type Width: LayoutBoundMarker;
//     type Height: LayoutBoundMarker;

//     type WidthIntrinsic: LayoutIntrinsicMarker;
//     type HeightIntrinsic: LayoutIntrinsicMarker;

//     fn unbounded(
//         self,
//     ) -> Box<
//         dyn AnyView<
//             Render = BoxedRenderObject<
//                 Unbounded,
//                 Unbounded,
//                 Self::WidthIntrinsic,
//                 Self::HeightIntrinsic,
//             >,
//         >,
//     >;
// }

// impl<Width, Height, WidthIntrinsic, HeightIntrinsic> BoxedViewExt
//     for Box<
//         dyn AnyView<
//             Render = Box<
//                 dyn AnyRenderObject<
//                     Width = Width,
//                     Height = Height,
//                     WidthIntrinsic = WidthIntrinsic,
//                     HeightIntrinsic = HeightIntrinsic,
//                 >,
//             >,
//         >,
//     >
// where
//     Width: LayoutBoundMarker,
//     Height: LayoutBoundMarker,
//     WidthIntrinsic: LayoutIntrinsicMarker,
//     HeightIntrinsic: LayoutIntrinsicMarker,
// {
//     type Width = Width;
//     type Height = Height;

//     type WidthIntrinsic = WidthIntrinsic;
//     type HeightIntrinsic = HeightIntrinsic;

//     fn unbounded(
//         self,
//     ) -> Box<
//         dyn AnyView<
//             Render = BoxedRenderObject<
//                 Unbounded,
//                 Unbounded,
//                 Self::WidthIntrinsic,
//                 Self::HeightIntrinsic,
//             >,
//         >,
//     > {
//         unsafe { std::mem::transmute(self) }
//     }
// }

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::{render_object::RenderLeaf, test_harness::TestHarness};

    use super::*;

    thread_local! {
        static MOUNT_COUNT: RefCell<usize> = const { RefCell::new(0) };
        static UPDATE_COUNT: RefCell<usize> = const { RefCell::new(0) };
    }

    pub struct TestView<T> {
        value: T,
    }

    impl<T> TestView<T> {
        pub fn new(value: T) -> Self {
            Self { value }
        }
    }

    impl<T> View for TestView<T>
    where
        T: Clone + 'static,
    {
        type Render = RenderLeaf;

        type State = T;

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            MOUNT_COUNT.with(|count| *count.borrow_mut() += 1);

            (vec![], self.value.clone())
        }

        fn update(&self, element: &mut Element, _: &Self, _: &mut UpdateCtx) {
            UPDATE_COUNT.with(|count| *count.borrow_mut() += 1);

            *element.state.downcast_mut::<Self>() = self.value.clone();
        }

        fn message(&self, _: &mut Element, _: MessageCtx) {}

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn mounting_dyn_views() {
        let harness = TestHarness::mount(&TestView::new(7_usize).as_dyn_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &7);
    }

    #[test]
    fn mounting_boxed_views() {
        let harness = TestHarness::mount(&TestView::new(1_usize).into_boxed_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &1);
    }

    #[test]
    fn updating_dyn_views() {
        let view = TestView::new(2_usize);

        let mut harness = TestHarness::mount(&view.as_dyn_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &2);

        harness.update(&view.as_dyn_view(), &TestView::new(9_usize).as_dyn_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &9);
    }

    #[test]
    fn updating_boxed_views() {
        let view = TestView::new(2_usize).into_boxed_view();

        let mut harness = TestHarness::mount(&view);

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &2);

        harness.update(&view, &TestView::new(9_usize).into_boxed_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &9);
    }

    #[test]
    fn replacing_dyn_views() {
        let view = TestView::new(2_usize);

        let mut harness = TestHarness::mount(&view.as_dyn_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &2);

        harness.update(&view.as_dyn_view(), &TestView::new(7_u8).as_dyn_view());

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(harness.root.state.downcast_ref::<TestView<u8>>(), &7);
    }

    #[test]
    fn replacing_boxed_views() {
        let view = TestView::new(2_usize).into_boxed_view();

        let mut harness = TestHarness::mount(&view);

        assert_eq!(harness.root.state.downcast_ref::<TestView<usize>>(), &2);

        harness.update(&view, &TestView::new(7_u8).into_boxed_view());

        assert_eq!(harness.root.state.downcast_ref::<TestView<u8>>(), &7);
    }
}
