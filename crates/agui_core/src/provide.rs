use std::{
    any::{Any, TypeId},
    rc::Rc,
};

use bon::Builder;

use crate::{
    context::{Dispatch, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, RoutingId, node::ElementNode},
    widget::Widget,
};

/// The values in scope for a subtree, each looked up by its type.
///
/// A value added with [`provide`](Self::provide) is visible through [`get`](Self::get) to the
/// subtree built under that call. Providing a type already in scope shadows the earlier value for
/// the new subtree, leaving the scope it extended untouched, so sibling subtrees keep seeing the
/// original.
#[derive(Default, Clone)]
pub struct ProvideScope {
    head: Option<Rc<ProvideNode>>,
}

struct ProvideNode {
    type_id: TypeId,
    value: Rc<dyn Any>,
    parent: Option<Rc<ProvideNode>>,
}

impl ProvideScope {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<T>(&self) -> Option<Rc<T>>
    where
        T: Any,
    {
        let target = TypeId::of::<T>();
        let mut node = self.head.as_deref();

        while let Some(current) = node {
            if current.type_id == target {
                return Rc::clone(&current.value).downcast::<T>().ok();
            }

            node = current.parent.as_deref();
        }

        None
    }

    pub fn provide<T>(&self, value: Rc<T>) -> ProvideScope
    where
        T: Any,
    {
        ProvideScope {
            head: Some(Rc::new(ProvideNode {
                type_id: TypeId::of::<T>(),
                value,
                parent: self.head.clone(),
            })),
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

            Dispatch::DependencyChanged(ctx) => {
                let value = Rc::clone(&self.value);
                ctx.with_provided(value, |ctx| {
                    self.child
                        .element
                        .dispatch(render, path, Dispatch::DependencyChanged(ctx));
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

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::{
        context::{Dispatch, UpdateCtx},
        element::{BuildBoundaryId, Element, RebuildBoundary, RoutingPath},
        pipeline::build::BuildOwner,
        provide::ProvideScope,
        test_fixtures::{Leaf, Transparent},
        test_harness::{TestTaskRunner, with_ctx, with_ctx_in},
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
    fn provided_value_reaches_a_dependency_changed_rebuild() {
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

        // A dependency-changed dispatch must re-thread the ancestor's provided value just like a
        // plain rebuild, so the woken descendant still reads it.
        with_ctx(|ctx| element.dispatch(&mut render, &[], Dispatch::DependencyChanged(ctx)));

        assert_eq!(
            seen.get(),
            Some(42),
            "a dependency-changed rebuild re-threads the ancestor's provided value"
        );
    }

    #[test]
    fn provided_value_survives_a_nested_build_boundary() {
        let boundary = Rc::new(Cell::new(None::<BuildBoundaryId>));
        let seen = Rc::new(Cell::new(None::<usize>));

        let captured = Rc::clone(&boundary);
        let recorder = Rc::clone(&seen);
        let widget = Provide::new(Rc::new(42_usize)).child(
            RebuildBoundary::new().child(
                Leaf::new()
                    .on_mount(move |ctx: &mut UpdateCtx| {
                        captured.set(ctx.build_scope().boundary());
                    })
                    .on_rebuild(move |ctx: &mut UpdateCtx| {
                        recorder.set(ctx.get_provided::<usize>().as_deref().copied());
                    }),
            ),
        );

        let mut tasks = TestTaskRunner::new();
        let (mut owner, _render) = BuildOwner::mount(widget, &mut tasks.scheduler());

        let boundary = boundary
            .get()
            .expect("the leaf mounted under the rebuild boundary");

        // The Provide sits above the boundary, so the value lives in the scope the boundary captured
        // at registration; a targeted flush into the boundary must re-enter with that scope.
        owner.request_dependency_change(&RoutingPath::new(boundary, Vec::new()));
        assert!(owner.flush(&mut tasks.scheduler()));

        assert_eq!(
            seen.get(),
            Some(42),
            "an ancestor's provided value reaches a descendant across a build boundary"
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
