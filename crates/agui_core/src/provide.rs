use std::{
    any::{Any, TypeId},
    hash::{BuildHasherDefault, Hasher},
    rc::Rc,
};

use imbl::shared_ptr::RcK;

#[derive(Default, Clone)]
pub struct ProvideScope {
    map: imbl::GenericHashMap<TypeId, Rc<dyn Any>, BuildHasherDefault<TypeIdHasher>, RcK>,
}

impl ProvideScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }

    pub fn provide<T>(&self, value: Rc<T>) -> ProvideScope
    where
        T: Any,
    {
        ProvideScope {
            map: self.map.update(TypeId::of::<T>(), value),
        }
    }
}

#[derive(Default)]
pub struct TypeIdHasher {
    value: u64,
}

impl Hasher for TypeIdHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // This expects to receive exactly one 64-bit value, and there’s no realistic chance of
        // that changing, but I don’t want to depend on something that isn’t expressly part of the
        // contract for safety. But I’m OK with release builds putting everything in one bucket
        // if it *did* change (and debug builds panicking).
        debug_assert_eq!(bytes.len(), 8);

        let _ = bytes
            .try_into()
            .map(|array| self.value = u64::from_ne_bytes(array));
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use std::{any::Any, fmt::Debug, rc::Rc};

    use crate::{
        context::{Dispatch, UpdateCtx},
        element::Element,
        provide::ProvideScope,
        render_object::RenderLeaf,
        routing_id::RoutingId,
        test_harness::TestHarness,
        view::View,
    };

    struct TestProviderView<T, Child> {
        value: Rc<T>,

        child: Child,
    }

    impl<T, Child> View for TestProviderView<T, Child>
    where
        T: Any,
        Child: View,
    {
        type Render = RenderLeaf;

        type State = ();

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (
                ctx.with_provided(Rc::clone(&self.value), |ctx| {
                    vec![Element::new(&self.child, ctx)]
                }),
                (),
            )
        }

        fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
            ctx.with_provided(Rc::clone(&self.value), |ctx| {
                element.child_mut(0, &old.child).update(&self.child, ctx)
            })
        }

        fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
            element.child_mut(0, &self.child).dispatch(path, action)
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    struct TestConsumerView<T> {
        expect: T,
    }

    impl<T> TestConsumerView<T> {
        pub fn new(expect: T) -> Self {
            Self { expect }
        }
    }

    impl<T> View for TestConsumerView<T>
    where
        T: Any + PartialEq + Debug,
    {
        type Render = RenderLeaf;

        type State = ();

        fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            assert_eq!(ctx.get_provided::<T>(), Some(&self.expect));

            (vec![], ())
        }

        fn update(&self, _: &mut Element, _: &Self, ctx: &mut UpdateCtx) {
            assert_eq!(ctx.get_provided::<T>(), Some(&self.expect));
        }

        fn create_render_object(&self, _: &Element) -> Self::Render {
            RenderLeaf::default()
        }

        fn update_render_object(&self, _: &Element, _: &mut Self::Render) {}
    }

    #[test]
    fn scope_can_provide_and_get_types() {
        let scope = ProvideScope::new();

        let scope = scope.provide::<usize>(Rc::new(1));

        assert_eq!(scope.get::<usize>(), Some(&1));
    }

    #[test]
    fn elements_can_provide_and_get_types() {
        let view_3 = TestProviderView {
            value: Rc::new(3_usize),

            child: TestConsumerView::new(3_usize),
        };

        let mut harness = TestHarness::mount(&view_3);

        let view_6 = TestProviderView {
            value: Rc::new(6_usize),

            child: TestConsumerView::new(6_usize),
        };

        harness.update(&view_3, &view_6);
    }

    #[test]
    fn nested_elements_to_provide_multiple_types() {
        let view = TestProviderView {
            value: Rc::new(3_usize),

            child: TestProviderView {
                value: Rc::new(6_i32),

                child: TestConsumerView::new(3_usize),
            },
        };

        let _ = TestHarness::mount(&view);
    }
}
