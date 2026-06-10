use std::{
    any::{Any, TypeId},
    hash::{BuildHasherDefault, Hasher},
    rc::Rc,
};

use bon::Builder;
use imbl::shared_ptr::RcK;

use crate::{
    context::{Dispatch, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, RoutingId, node::ElementNode},
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

    pub fn get<T>(&self) -> Option<Rc<T>>
    where
        T: Any,
    {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|rc| Rc::clone(rc).downcast::<T>().ok())
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
    type Element = ProvideElement<V, Child::Element>;

    type Render = Child::Render;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (element, render_object) =
            ctx.with_provided(Rc::clone(&self.value), |ctx| self.child.create(ctx));

        (
            ProvideElement {
                child: ElementNode::new(element),
                value: self.value,
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
        element.value = Rc::clone(&self.value);

        ctx.with_provided(self.value, |ctx| {
            self.child
                .update(&mut element.child.element, render_object, ctx);
        });
    }
}

/// The [`Element`] of a [`Provide`], re-applying the provided value when a rebuild reaches its subtree.
pub struct ProvideElement<V, C> {
    child: ElementNode<C>,
    value: Rc<V>,
}

impl<V, C> Element for ProvideElement<V, C>
where
    V: Any,
    C: Element,
    C::Render: Sized,
{
    type Render = C::Render;

    fn dispatch(&mut self, render: &mut C::Render, path: &[RoutingId], action: Dispatch) {
        match action {
            Dispatch::Rebuild(ctx) => {
                let value = Rc::clone(&self.value);
                ctx.with_provided(value, |ctx| {
                    self.child
                        .element
                        .dispatch(render, path, Dispatch::Rebuild(ctx));
                });
            }

            Dispatch::Message(ctx) => {
                self.child
                    .element
                    .dispatch(render, path, Dispatch::Message(ctx));
            }
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node(format!("Provide<{}>", Diagnostics::short_type_name::<V>()))
            .child(|d| self.child.element.describe(d))
            .finish()
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
    use std::{cell::Cell, rc::Rc};

    use crate::{
        context::{Dispatch, UpdateCtx},
        element::Element,
        provide::ProvideScope,
        test_fixtures::{Leaf, Transparent},
        test_harness::{with_ctx, with_ctx_in},
        widget::Widget,
    };

    use super::Provide;

    #[test]
    fn scope_can_provide_and_get_types() {
        let scope = ProvideScope::new().provide::<usize>(Rc::new(1));

        assert_eq!(scope.get::<usize>(), Some(Rc::new(1)));
    }

    /// A cell that records the `usize` a hook saw, plus a recorder closure to install on a leaf.
    fn recorder() -> (Rc<Cell<Option<usize>>>, impl Fn(&mut UpdateCtx) + 'static) {
        let seen = Rc::new(Cell::new(None));
        let recorder = Rc::clone(&seen);
        (seen, move |ctx: &mut UpdateCtx| {
            recorder.set(ctx.get_provided::<usize>().as_deref().copied());
        })
    }

    #[test]
    fn provide_exposes_value_to_subtree_on_mount() {
        let (seen, record) = recorder();

        with_ctx(|ctx| {
            Provide::new(Rc::new(42_usize))
                .child(Leaf::new().on_mount(record))
                .create(ctx);
        });

        assert_eq!(
            seen.get(),
            Some(42),
            "the leaf's on_mount must run and see the value"
        );
    }

    #[test]
    fn nested_provides_expose_multiple_types() {
        let seen_usize = Rc::new(Cell::new(None));
        let seen_i32 = Rc::new(Cell::new(None));

        let (ru, ri) = (Rc::clone(&seen_usize), Rc::clone(&seen_i32));
        with_ctx(|ctx| {
            Provide::new(Rc::new(3_usize))
                .child(
                    Provide::new(Rc::new(6_i32)).child(Leaf::new().on_mount(move |ctx| {
                        ru.set(ctx.get_provided::<usize>().as_deref().copied());
                        ri.set(ctx.get_provided::<i32>().as_deref().copied());
                    })),
                )
                .create(ctx);
        });

        assert_eq!(seen_usize.get(), Some(3));
        assert_eq!(seen_i32.get(), Some(6));
    }

    #[test]
    fn providing_same_type_twice_returns_latest() {
        let (seen, record) = recorder();

        with_ctx(|ctx| {
            Provide::new(Rc::new(1_usize))
                .child(Provide::new(Rc::new(2_usize)).child(Leaf::new().on_mount(record)))
                .create(ctx);
        });

        assert_eq!(seen.get(), Some(2));
    }

    #[test]
    fn update_reprovides_the_new_value() {
        let (mounted, record_mount) = recorder();
        let (mut element, mut render) = with_ctx(|ctx| {
            Provide::new(Rc::new(3_usize))
                .child(Leaf::new().on_mount(record_mount))
                .create(ctx)
        });
        assert_eq!(
            mounted.get(),
            Some(3),
            "the leaf's on_mount must run and see the value"
        );

        let (updated, record_update) = recorder();
        with_ctx(|ctx| {
            Provide::new(Rc::new(6_usize))
                .child(Leaf::new().on_update(record_update))
                .update(&mut element, &mut render, ctx);
        });
        assert_eq!(
            updated.get(),
            Some(6),
            "the leaf's on_update must run and see the new value"
        );
    }

    #[test]
    fn provided_value_survives_a_targeted_rebuild_below_it() {
        let seen = Rc::new(Cell::new(None::<usize>));

        let recorder = Rc::clone(&seen);
        let (mut element, mut render) = with_ctx(|ctx| {
            Provide::new(Rc::new(42_usize))
                .child(Transparent {
                    child: Leaf::new().on_rebuild(move |ctx| {
                        recorder.set(ctx.get_provided::<usize>().as_deref().copied());
                    }),
                })
                .create(ctx)
        });

        // The transparent single child pushes no routing id, so the leaf sits at the empty path.
        with_ctx(|ctx| element.dispatch(&mut render, &[], Dispatch::Rebuild(ctx)));

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
        let (mut element, mut render) = with_ctx(|ctx| {
            Transparent {
                child: Leaf::new().on_rebuild(move |ctx| {
                    recorder.set(ctx.get_provided::<usize>().as_deref().copied());
                }),
            }
            .create(ctx)
        });

        let scope = ProvideScope::new().provide::<usize>(Rc::new(7_usize));
        with_ctx_in(&scope, |ctx| {
            element.dispatch(&mut render, &[], Dispatch::Rebuild(ctx));
        });

        assert_eq!(
            seen.get(),
            Some(7),
            "a value provided at the root scope reaches a targeted rebuild"
        );
    }
}
