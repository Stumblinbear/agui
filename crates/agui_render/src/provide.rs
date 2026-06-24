use std::{
    any::{Any, TypeId},
    cell::RefCell,
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};

use bon::Builder;
use rustc_hash::FxHashSet;

use agui_core::tree::{NodeHandle, Slot};

use crate::{
    context::{CreateCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    render_object::node::RenderObjectPtr,
    widget::Widget,
};

/// The values in scope for a node: the chain of [`Provide`]s above it.
///
/// Reading walks the chain for the nearest value of the requested type. The scope is a cheap-to-copy borrow
/// into the ancestor elements that hold the values, not an owned structure.
#[derive(Clone, Copy, Default)]
pub struct ProvideScope {
    head: Option<NonNull<ProvideNode>>,
}

/// One link in the scope chain, held inline by a [`Provide`]'s element. `parent` points at the next value in
/// scope and is wired when the element mounts, once its address is final.
struct ProvideNode {
    cell: ProvideCell,
    parent: Option<NonNull<ProvideNode>>,
}

/// The live value a [`Provide`] holds and the handles that read it.
///
/// Owned by the `Provide`'s element; readers reach it through the scope chain rather than a shared pointer.
/// The value is replaced in place when the `Provide` is given a different one, and every reader is recorded
/// so that replacement can mark it for a dependency-change rebuild.
struct ProvideCell {
    type_id: TypeId,
    value: RefCell<Rc<dyn Any>>,
    dependents: RefCell<FxHashSet<NodeHandle>>,
}

impl ProvideCell {
    fn new<V: Any>(value: V) -> Self {
        Self {
            type_id: TypeId::of::<V>(),
            value: RefCell::new(Rc::new(value)),
            dependents: RefCell::new(FxHashSet::default()),
        }
    }

    fn read(&self) -> Rc<dyn Any> {
        Rc::clone(&self.value.borrow())
    }

    fn depend(&self, dependent: NodeHandle) {
        self.dependents.borrow_mut().insert(dependent);
    }

    /// Replaces the held value and returns the readers to notify, clearing the recorded set.
    fn replace(&self, value: Rc<dyn Any>) -> Vec<NodeHandle> {
        *self.value.borrow_mut() = value;
        self.dependents.borrow_mut().drain().collect()
    }
}

impl ProvideScope {
    /// Reads the nearest value of type `T` in scope, if one is present, without recording a dependency.
    pub fn get<T: Any>(&self) -> Option<Rc<T>> {
        self.find(TypeId::of::<T>())
            .and_then(|cell| cell.read().downcast::<T>().ok())
    }

    /// Reads the nearest value of type `T` and records `dependent` against it, so a later change to that
    /// value marks `dependent` for a dependency-change rebuild.
    pub(crate) fn get_and_depend<T: Any>(self, dependent: NodeHandle) -> Option<Rc<T>> {
        let cell = self.find(TypeId::of::<T>())?;
        cell.depend(dependent);

        cell.read().downcast::<T>().ok()
    }

    fn find(&self, type_id: TypeId) -> Option<&ProvideCell> {
        let mut node = self.head;

        while let Some(ptr) = node {
            // SAFETY: each chain pointer addresses a live ancestor `Provide`'s node. A `Provide` outlives
            // every descendant that can hold this scope, and the node sits in its own allocation, never
            // borrowed `&mut` after mount, so this shared read is sound even while an ancestor is being
            // reconciled through `&mut`.
            let current = unsafe { ptr.as_ref() };

            if current.cell.type_id == type_id {
                return Some(&current.cell);
            }

            node = current.parent;
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

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let child_element = self.child.create(ctx);

        ProvideElement {
            node: Box::new(ProvideNode {
                cell: ProvideCell::new(self.value),
                parent: None,
            }),
            child: Slot::new(child_element),
            _value: PhantomData,
        }
    }

    fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut Self::Element) {
        // A shared borrow of the boxed node (the value changes through the cell's interior mutability), so it
        // never takes `&mut` to the node and never invalidates a descendant's scope pointer into it.
        let cell = &element.node.cell;

        let changed = cell
            .read()
            .downcast_ref::<V>()
            .is_none_or(|current| *current != self.value);

        if changed {
            let dependents = cell.replace(Rc::new(self.value));
            for dependent in dependents {
                ctx.mark_dependency_changed(dependent);
            }
        }

        let scope = element.scope();
        // SAFETY: `element.child` is the element's own slot. Provide is transparent: it only extends the
        // scope, reconciling the child in place.
        ctx.with_scope(scope, |ctx| unsafe {
            ctx.with_child(&mut element.child, |child, ctx| {
                self.child.update(ctx, child);
            });
        });
    }
}

/// The [`Element`] of a [`Provide`]. It holds the value inline and the child it wraps, and exposes the value
/// to its subtree by extending the scope when it mounts.
pub struct ProvideElement<V, C> {
    /// Boxed into its own allocation so a descendant's scope pointer into it survives this element being
    /// reconciled through `&mut`, which retags only the element's own allocation, not the node's.
    node: Box<ProvideNode>,
    child: Slot<C>,
    _value: PhantomData<fn() -> V>,
}

impl<V, C> ProvideElement<V, C> {
    /// The scope this element hands to its subtree: the chain rooted at its own node.
    fn scope(&self) -> ProvideScope {
        ProvideScope {
            // Points into the node's own allocation, which a `&mut` to this element does not retag, so a
            // descendant's read through this pointer stays valid across the element's reconciles.
            head: Some(NonNull::from(&*self.node)),
        }
    }
}

impl<V, C> Element for ProvideElement<V, C>
where
    V: Any,
    C: Element,
{
    type Render = C::Render;

    fn render_object_mut(&mut self) -> &mut Self::Render {
        self.child.get_mut().render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
        self.child.get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.node.parent = ctx.provide().head;

        let scope = self.scope();
        // SAFETY: `self.child` is our own slot, and `self.node` is pinned now that this element is mounted,
        // so the scope it hands down stays valid for the whole subtree's life.
        ctx.with_scope(scope, |ctx| unsafe { ctx.mount(&mut self.child) });
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node(format!("Provide<{}>", Diagnostics::short_type_name::<V>()))
            .child(|d| self.child.get().describe(d))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use super::Provide;
    use crate::{test_fixtures::Leaf, test_harness::WidgetTester};

    #[test]
    fn a_descendant_reads_a_provided_value_at_mount() {
        let read = Rc::new(Cell::new(None));
        let recorder = Rc::clone(&read);
        let leaf = Leaf::new()
            .on_mount(move |ctx| recorder.set(ctx.get_provided::<usize>().as_deref().copied()));

        let _tester = WidgetTester::mount(Provide::new(42usize).child(leaf));

        assert_eq!(
            read.get(),
            Some(42),
            "the descendant saw the provided value in scope"
        );
    }

    #[test]
    fn an_absent_provided_value_reads_none() {
        let read = Rc::new(Cell::new(Some(0)));
        let recorder = Rc::clone(&read);
        let leaf = Leaf::new()
            .on_mount(move |ctx| recorder.set(ctx.get_provided::<usize>().as_deref().copied()));

        let _tester = WidgetTester::mount(leaf);

        assert_eq!(read.get(), None, "no provider in scope, no value");
    }

    // A descendant that depends on the provided `usize` at mount and flags when its dependency-change hook
    // runs. `Leaf` routes `dependency_changed` to `on_rebuild`.
    fn dependent_reader(fired: Rc<Cell<bool>>) -> Leaf {
        Leaf::new()
            .on_mount(|ctx| {
                ctx.build(|ctx| {
                    ctx.depend_on_provided::<usize>();
                });
            })
            .on_rebuild(move |_| fired.set(true))
    }

    #[test]
    fn changing_a_provided_value_runs_a_dependents_dependency_change_hook() {
        let fired = Rc::new(Cell::new(false));

        let mut tester =
            WidgetTester::mount(Provide::new(1usize).child(dependent_reader(Rc::clone(&fired))));
        assert!(
            !fired.get(),
            "mount registers the dependency but runs no change hook"
        );

        tester.rebuild(Provide::new(2usize).child(dependent_reader(Rc::clone(&fired))));
        assert!(
            fired.get(),
            "changing the value ran the dependent's dependency-change hook"
        );
    }
}
