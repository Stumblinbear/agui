use std::{
    any::{Any, TypeId},
    hash::{BuildHasherDefault, Hasher},
    rc::Rc,
};

use bon::Builder;
use imbl::shared_ptr::RcK;

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{RoutingId, SingleChildElement},
    widget::Widget,
};

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

/// A widget that makes one value available to its subtree.
///
/// # Examples
///
/// ```ignore
/// Provide::new(Rc::new(theme)).child(page)
/// ```
#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct Provide<V, Child> {
    #[builder(start_fn)]
    value: Rc<V>,

    #[builder(finish_fn)]
    child: Child,
}

impl<V, Child> Widget for Provide<V, Child>
where
    V: Any,
    Child: Widget,
{
    type Element = SingleChildElement<Child::Element>;

    type Render = Child::Render;

    fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
        ctx.with_provided(Rc::clone(&self.value), |ctx| {
            SingleChildElement::new(&self.child, ctx)
        })
    }

    fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
        ctx.with_provided(Rc::clone(&self.value), |ctx| {
            element.update(&self.child, &old.child, ctx);
        });
    }

    fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
        match action {
            Dispatch::Rebuild(ctx) => ctx.with_provided(Rc::clone(&self.value), |ctx| {
                element.dispatch(&self.child, path, Dispatch::Rebuild(ctx));
            }),

            action @ Dispatch::Message(_) => element.dispatch(&self.child, path, action),
        }
    }

    fn create_render_object(&self, element: &Self::Element) -> Self::Render {
        element.create_render_object(&self.child)
    }

    fn update_render_object(&self, element: &Self::Element, render_object: &mut Self::Render) {
        element.update_render_object(&self.child, render_object);
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
    use std::{any::Any, cell::Cell, rc::Rc};

    use crate::{
        context::{Dispatch, UpdateCtx},
        element::{RoutingId, SingleChildElement},
        provide::ProvideScope,
        test_fixtures::{Leaf, Transparent},
        test_harness::TestHarness,
        widget::Widget,
    };

    use super::Provide;

    struct TestProviderWidget<T, Child> {
        value: Rc<T>,

        child: Child,
    }

    impl<T, Child> Widget for TestProviderWidget<T, Child>
    where
        T: Any,
        Child: Widget,
    {
        type Element = SingleChildElement<Child::Element>;

        type Render = ();

        fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
            ctx.with_provided(Rc::clone(&self.value), |ctx| {
                SingleChildElement::new(&self.child, ctx)
            })
        }

        fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
            ctx.with_provided(Rc::clone(&self.value), |ctx| {
                element.update(&self.child, &old.child, ctx);
            });
        }

        fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
            element.dispatch(&self.child, path, action);
        }

        fn create_render_object(&self, _: &Self::Element) -> Self::Render {}

        fn update_render_object(&self, _: &Self::Element, (): &mut Self::Render) {}
    }

    #[test]
    fn scope_can_provide_and_get_types() {
        let scope = ProvideScope::new();

        let scope = scope.provide::<usize>(Rc::new(1));

        assert_eq!(scope.get::<usize>(), Some(&1));
    }

    #[test]
    fn elements_can_provide_and_get_types() {
        let widget_3 = TestProviderWidget {
            value: Rc::new(3_usize),
            child: Leaf::new().on_mount(|ctx| assert_eq!(ctx.get_provided::<usize>(), Some(&3))),
        };

        let mut harness = TestHarness::mount(&widget_3);

        let widget_6 = TestProviderWidget {
            value: Rc::new(6_usize),
            child: Leaf::new().on_update(|ctx| assert_eq!(ctx.get_provided::<usize>(), Some(&6))),
        };

        harness.update(&widget_3, &widget_6);
    }

    #[test]
    fn nested_elements_to_provide_multiple_types() {
        let widget = TestProviderWidget {
            value: Rc::new(3_usize),
            child: TestProviderWidget {
                value: Rc::new(6_i32),
                child: Leaf::new().on_mount(|ctx| {
                    assert_eq!(ctx.get_provided::<usize>(), Some(&3));
                    assert_eq!(ctx.get_provided::<i32>(), Some(&6));
                }),
            },
        };

        let _ = TestHarness::mount(&widget);
    }

    #[test]
    fn providing_same_type_twice_returns_latest() {
        let widget = TestProviderWidget {
            value: Rc::new(1_usize),
            child: TestProviderWidget {
                value: Rc::new(2_usize),
                child: Leaf::new()
                    .on_mount(|ctx| assert_eq!(ctx.get_provided::<usize>(), Some(&2))),
            },
        };

        let _ = TestHarness::mount(&widget);
    }

    #[test]
    fn provide_widget_exposes_value_to_subtree_on_mount() {
        let widget = Provide::new(Rc::new(42_usize))
            .child(Leaf::new().on_mount(|ctx| assert_eq!(ctx.get_provided::<usize>(), Some(&42))));

        let _ = TestHarness::mount(&widget);
    }

    #[test]
    fn provided_value_survives_a_targeted_rebuild_below_it() {
        let seen = Rc::new(Cell::new(None::<usize>));

        let recorder = Rc::clone(&seen);
        let widget = Provide::new(Rc::new(42_usize)).child(Transparent {
            child: Leaf::new()
                .on_rebuild(move |ctx| recorder.set(ctx.get_provided::<usize>().copied())),
        });

        let mut harness = TestHarness::mount(&widget);

        // The transparent single child pushes no routing id, so the leaf sits at the empty path.
        harness.dispatch_rebuild(&widget, &[]);

        assert_eq!(
            seen.get(),
            Some(42),
            "an ancestor's provided value is still visible when only the descendant rebuilds"
        );
    }

    #[test]
    fn root_provided_value_survives_a_targeted_rebuild() {
        let seen = Rc::new(Cell::new(None::<usize>));

        let recorder = Rc::clone(&seen);
        let widget = Transparent {
            child: Leaf::new()
                .on_rebuild(move |ctx| recorder.set(ctx.get_provided::<usize>().copied())),
        };

        let mut harness = TestHarness::mount(&widget);
        harness.provide_scope = harness.provide_scope.provide::<usize>(Rc::new(7));

        harness.dispatch_rebuild(&widget, &[]);

        assert_eq!(
            seen.get(),
            Some(7),
            "a value provided at the root scope reaches a targeted rebuild"
        );
    }
}
