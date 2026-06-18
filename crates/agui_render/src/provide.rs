use std::{
    any::{Any, TypeId},
    cell::RefCell,
    marker::PhantomData,
    rc::Rc,
};

use bon::Builder;
use rustc_hash::FxHashSet;

use crate::{
    context::{Dispatch, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, RoutingPath, RoutingTarget, node::ElementNode},
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
    cell: Rc<ProvideCell>,
    parent: Option<Rc<ProvideNode>>,
}

/// The live value a [`Provide`] holds and the addresses that read it.
///
/// The value is replaced in place when the [`Provide`] is given a different one, and every reader is
/// recorded so that replacement can mark them for a dependency-change rebuild.
pub(crate) struct ProvideCell {
    type_id: TypeId,
    value: RefCell<Rc<dyn Any>>,
    dependents: RefCell<FxHashSet<RoutingTarget>>,
}

impl ProvideCell {
    pub(crate) fn new<V: Any>(value: V) -> Rc<Self> {
        Rc::new(Self {
            type_id: TypeId::of::<V>(),
            value: RefCell::new(Rc::new(value)),
            dependents: RefCell::new(FxHashSet::default()),
        })
    }

    fn read(&self) -> Rc<dyn Any> {
        Rc::clone(&self.value.borrow())
    }

    fn depend(&self, dependent: &RoutingTarget) {
        self.dependents.borrow_mut().insert(dependent.clone());
    }

    /// Replaces the held value and returns the readers to notify, clearing them so they re-record
    /// themselves as they rebuild.
    fn replace(&self, value: Rc<dyn Any>) -> Vec<RoutingTarget> {
        *self.value.borrow_mut() = value;
        self.dependents.borrow_mut().drain().collect()
    }
}

impl ProvideScope {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the nearest value of type `T`, if one is in scope, without recording a dependency on it.
    pub fn get<T: Any>(&self) -> Option<Rc<T>> {
        self.find(TypeId::of::<T>())
            .and_then(|cell| cell.read().downcast::<T>().ok())
    }

    /// Reads the nearest value of type `T` and records `dependent` against it, so a later change to
    /// that value marks `dependent` for a dependency-change rebuild.
    pub(crate) fn get_and_depend<T: Any>(&self, dependent: &RoutingTarget) -> Option<Rc<T>> {
        let cell = self.find(TypeId::of::<T>())?;
        cell.depend(dependent);

        cell.read().downcast::<T>().ok()
    }

    /// Extends this scope with a fresh `value` in scope, owned by the scope rather than a [`Provide`].
    /// A convenience for building a scope standalone; a change to the value is never observed, since no
    /// `Provide` re-provides it.
    pub fn provide<V: Any>(&self, value: V) -> ProvideScope {
        self.with_cell(ProvideCell::new(value))
    }

    /// Extends this scope with `cell` in scope for the subtree built under the returned scope.
    pub(crate) fn with_cell(&self, cell: Rc<ProvideCell>) -> ProvideScope {
        ProvideScope {
            head: Some(Rc::new(ProvideNode {
                cell,
                parent: self.head.clone(),
            })),
        }
    }

    fn find(&self, type_id: TypeId) -> Option<&Rc<ProvideCell>> {
        let mut node = self.head.as_deref();

        while let Some(current) = node {
            if current.cell.type_id == type_id {
                return Some(&current.cell);
            }

            node = current.parent.as_deref();
        }

        None
    }
}

/// A widget that makes one value available to its subtree.
///
/// # Examples
///
/// ```ignore
/// Provide::new(theme).child(page)
/// ```
#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct Provide<V, Child> {
    #[builder(start_fn)]
    value: V,

    #[builder(finish_fn)]
    child: Child,
}

impl<V, Child> Widget for Provide<V, Child>
where
    V: PartialEq + 'static,
    Child: Widget,
{
    type Element = ProvideElement<V, Child::Element>;

    type Render = Child::Render;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let cell = ProvideCell::new(self.value);

        let (element, render_object) =
            ctx.with_provided(Rc::clone(&cell), |ctx| self.child.create(ctx));

        (
            ProvideElement {
                child: ElementNode::new(element),
                cell,
                _value: PhantomData,
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
        let Provide { value, child } = self;
        let cell = Rc::clone(&element.cell);

        let changed = cell
            .read()
            .downcast_ref::<V>()
            .is_none_or(|current| *current != value);

        if changed {
            for dependent in cell.replace(Rc::new(value)) {
                ctx.mark_dependency_changed(&dependent);
            }
        }

        ctx.with_provided(cell, |ctx| {
            child.update(&mut element.child.element, render_object, ctx);
        });
    }
}

/// The [`Element`] of a [`Provide`], re-applying the provided value when a rebuild reaches its subtree.
pub struct ProvideElement<V, C> {
    child: ElementNode<C>,
    cell: Rc<ProvideCell>,
    _value: PhantomData<fn() -> V>,
}

impl<V, C> Element for ProvideElement<V, C>
where
    V: Any,
    C: Element,
    C::Render: Sized,
{
    type Render = C::Render;

    fn dispatch(&mut self, render: &mut C::Render, path: &RoutingPath, action: Dispatch) {
        match action {
            Dispatch::Rebuild(ctx) => {
                ctx.with_provided(Rc::clone(&self.cell), |ctx| {
                    self.child
                        .element
                        .dispatch(render, path, Dispatch::Rebuild(ctx));
                });
            }

            Dispatch::DependencyChanged(ctx) => {
                ctx.with_provided(Rc::clone(&self.cell), |ctx| {
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
        context::{Dispatch, MessageCtx, UpdateCtx},
        element::{Element, RoutingPath, RoutingTarget},
        pipeline::{
            PipelineOwner,
            build::{BuildBoundaryId, RebuildBoundary},
        },
        provide::ProvideScope,
        test_fixtures::{Leaf, Transparent},
        test_harness::TestCtx,
    };

    use super::Provide;

    #[test]
    fn scope_can_provide_and_get_types() {
        let scope = ProvideScope::new().provide(1_usize);

        assert_eq!(scope.get::<usize>(), Some(Rc::new(1)));
    }

    /// A cell that records the `usize` a hook saw, plus a recorder closure to install on a leaf.
    fn recorder() -> (Rc<Cell<Option<usize>>>, impl Fn(&mut UpdateCtx) + 'static) {
        let seen = Rc::new(Cell::new(None));
        let recorder = Rc::clone(&seen);
        (seen, move |ctx: &mut UpdateCtx| {
            recorder.set(ctx.depend_on_provided::<usize>().as_deref().copied());
        })
    }

    #[test]
    fn provide_exposes_value_to_subtree_on_mount() {
        let (seen, record) = recorder();

        TestCtx::new().create(Provide::new(42_usize).child(Leaf::new().on_mount(record)));

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
        TestCtx::new().create(Provide::new(3_usize).child(Provide::new(6_i32).child(
            Leaf::new().on_mount(move |ctx| {
                ru.set(ctx.depend_on_provided::<usize>().as_deref().copied());
                ri.set(ctx.depend_on_provided::<i32>().as_deref().copied());
            }),
        )));

        assert_eq!(seen_usize.get(), Some(3));
        assert_eq!(seen_i32.get(), Some(6));
    }

    #[test]
    fn providing_same_type_twice_returns_latest() {
        let (seen, record) = recorder();

        TestCtx::new().create(
            Provide::new(1_usize).child(Provide::new(2_usize).child(Leaf::new().on_mount(record))),
        );

        assert_eq!(seen.get(), Some(2));
    }

    #[test]
    fn update_reprovides_the_new_value() {
        let (mounted, record_mount) = recorder();
        let (mut element, mut render) =
            TestCtx::new().create(Provide::new(3_usize).child(Leaf::new().on_mount(record_mount)));
        assert_eq!(
            mounted.get(),
            Some(3),
            "the leaf's on_mount must run and see the value"
        );

        let (updated, record_update) = recorder();
        TestCtx::new().update(
            Provide::new(6_usize).child(Leaf::new().on_update(record_update)),
            &mut element,
            &mut render,
        );
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
        let (mut element, mut render) =
            TestCtx::new().create(Provide::new(42_usize).child(Transparent {
                child: Leaf::new().on_rebuild(move |ctx| {
                    recorder.set(ctx.depend_on_provided::<usize>().as_deref().copied());
                }),
            }));

        // The transparent single child pushes no routing id, so the leaf sits at the empty path.
        TestCtx::new().run(|ctx| {
            element.dispatch(&mut render, RoutingPath::new(&[]), Dispatch::Rebuild(ctx));
        });

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
        let (mut element, mut render) =
            TestCtx::new().create(Provide::new(42_usize).child(Transparent {
                child: Leaf::new().on_rebuild(move |ctx| {
                    recorder.set(ctx.depend_on_provided::<usize>().as_deref().copied());
                }),
            }));

        // A dependency-changed dispatch must re-thread the ancestor's provided value just like a
        // plain rebuild, so the woken descendant still reads it.
        TestCtx::new().run(|ctx| {
            element.dispatch(
                &mut render,
                RoutingPath::new(&[]),
                Dispatch::DependencyChanged(ctx),
            );
        });

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
        let widget = Provide::new(42_usize).child(
            RebuildBoundary::new().child(
                Leaf::new()
                    .on_mount(move |ctx: &mut UpdateCtx| {
                        captured.set(ctx.build_scope().boundary());
                    })
                    .on_message(MessageCtx::request_rebuild)
                    .on_rebuild(move |ctx: &mut UpdateCtx| {
                        recorder.set(ctx.depend_on_provided::<usize>().as_deref().copied());
                    }),
            ),
        );

        let mut tasks = TestCtx::new();
        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        let boundary = boundary
            .get()
            .expect("the leaf mounted under the rebuild boundary");

        // The Provide sits above the boundary, so the value lives in the scope the boundary captured
        // at registration; a targeted rebuild into the boundary must re-enter with that scope.
        owner.dispatch_message(&RoutingTarget::new(boundary, Vec::new()), Box::new(()));
        assert!(owner.flush_build(&mut tasks.scheduler()));

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
        let (mut element, mut render) = TestCtx::new().create(Transparent {
            child: Leaf::new().on_rebuild(move |ctx| {
                recorder.set(ctx.depend_on_provided::<usize>().as_deref().copied());
            }),
        });

        TestCtx::new().with_provided(7_usize).run(|ctx| {
            element.dispatch(&mut render, RoutingPath::new(&[]), Dispatch::Rebuild(ctx));
        });

        assert_eq!(
            seen.get(),
            Some(7),
            "a value provided at the root scope reaches a targeted rebuild"
        );
    }
}
