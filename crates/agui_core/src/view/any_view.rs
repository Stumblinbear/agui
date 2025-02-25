use std::{any::Any, rc::Rc, sync::Arc};

use crate::{
    context::{MessageCtx, UpdateCtx},
    element::{Element, ElementState},
    render_object::{AnyRenderObject, RenderObject},
    view::{MountView, View},
};

pub trait AnyView {
    type Render: RenderObject;

    fn as_any(&self) -> &dyn Any;

    fn view_name(&self) -> &str;

    fn dyn_is_same_type(&self, other: &dyn AnyView<Render = Self::Render>) -> bool;

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

    fn dyn_is_same_type(&self, other: &dyn AnyView<Render = Self::Render>) -> bool {
        other.as_any().is::<Self>()
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

                fn is_same_type(&self, other: &Self) -> bool {
                    (**self).dyn_is_same_type(&**other)
                }

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
{
    type Render = Box<
        dyn AnyRenderObject<
            Width = <T::Render as RenderObject>::Width,
            Height = <T::Render as RenderObject>::Height,
            WidthIntrinsic = <T::Render as RenderObject>::WidthIntrinsic,
            HeightIntrinsic = <T::Render as RenderObject>::HeightIntrinsic,
        >,
    >;

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
        Box::new(self.inner.dyn_create_render_object(element))
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        if let Some(render_object) = render_object.as_any_mut().downcast_mut::<T::Render>() {
            self.inner.update_render_object(element, render_object);
        } else {
            *render_object = Box::new(self.inner.dyn_create_render_object(element));
        }
    }
}

#[allow(type_alias_bounds)]
pub type BoxedRenderObject<RO: RenderObject> = Box<
    dyn AnyRenderObject<
        Width = RO::Width,
        Height = RO::Height,
        WidthIntrinsic = RO::WidthIntrinsic,
        HeightIntrinsic = RO::HeightIntrinsic,
    >,
>;

#[allow(type_alias_bounds)]
pub type BoxedView<V: View> = Box<dyn AnyView<Render = BoxedRenderObject<V::Render>>>;

pub trait AsAnyView: View + 'static {
    fn as_dyn_view(&self) -> &(dyn AnyView<Render = Self::Render>)
    where
        Self: Sized,
    {
        self
    }

    fn into_boxed_view(self) -> BoxedView<Self>
    where
        Self: Sized,
    {
        Box::new(AnyViewWrapper { inner: self })
    }
}

impl<T: 'static> AsAnyView for T where T: View {}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::VecDeque, sync::mpsc};

    use crate::render_object::RenderLeaf;

    use super::*;

    thread_local! {
        static MOUNT_COUNT: RefCell<usize> = const { RefCell::new(0) };
        static UPDATE_COUNT: RefCell<usize> = const { RefCell::new(0) };
    }

    pub struct TestView<T> {
        value: T,
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
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let element = Element::new(
            &TestView { value: 7_usize }.as_dyn_view(),
            &mut UpdateCtx::new(&tx, &mut path),
        );

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &7);
    }

    #[test]
    fn mounting_boxed_views() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let element = Element::new(
            &TestView { value: 1_usize }.into_boxed_view(),
            &mut UpdateCtx::new(&tx, &mut path),
        );

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &1);
    }

    #[test]
    fn updating_dyn_views() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let view = TestView { value: 2_usize }.as_dyn_view();

        let mut element = Element::new(&view, &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &2);

        element
            .as_mut(&TestView { value: 9_usize }.as_dyn_view())
            .update(&view, &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &9);
    }

    #[test]
    fn updating_boxed_views() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let view = TestView { value: 2_usize }.into_boxed_view();

        let mut element = Element::new(&view, &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &2);

        element
            .as_mut(&TestView { value: 9_usize }.into_boxed_view())
            .update(&view, &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &9);
    }

    #[test]
    fn replacing_dyn_views() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let view = TestView { value: 2_usize };

        let mut element = Element::new(&view.as_dyn_view(), &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 1);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &2);

        element
            .as_mut(&TestView { value: 7_u8 }.as_dyn_view())
            .update(&view.as_dyn_view(), &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(MOUNT_COUNT.with(|count| *count.borrow()), 2);
        assert_eq!(UPDATE_COUNT.with(|count| *count.borrow()), 0);
        assert_eq!(element.state.downcast_ref::<TestView<u8>>(), &7);
    }

    #[test]
    fn replacing_boxed_views() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let view = TestView { value: 2_usize }.into_boxed_view();

        let mut element = Element::new(&view, &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(element.state.downcast_ref::<TestView<usize>>(), &2);

        element
            .as_mut(&TestView { value: 7_u8 }.into_boxed_view())
            .update(&view, &mut UpdateCtx::new(&tx, &mut path));

        assert_eq!(element.state.downcast_ref::<TestView<u8>>(), &7);
    }
}
