use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ptr::NonNull,
    rc::Rc,
};

use rustc_hash::FxHashSet;

use crate::tree::NodeHandle;

/// The values in scope for a node: the chain of provided values above it.
///
/// Reading walks the chain for the nearest value of the requested type. The scope is a cheap-to-copy borrow
/// into the ancestor nodes that hold the values, not an owned structure.
#[derive(Clone, Copy, Default)]
pub struct ProvideScope {
    head: Option<NonNull<ProvideNode>>,
}

/// One link in the scope chain, held inline by the element that provides a value. `parent` points at the next
/// value in scope and is wired with [`set_parent`](Self::set_parent) when the element mounts, once its address
/// is final.
pub struct ProvideNode {
    cell: ProvideCell,
    parent: Option<NonNull<ProvideNode>>,
}

/// The live value a node holds and the handles that read it.
///
/// Owned by the providing element; readers reach it through the scope chain rather than a shared pointer. The
/// value is replaced in place when the element is given a different one, and every reader is recorded so that
/// replacement can mark it for a dependency-change rebuild.
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
    pub fn get_and_depend<T: Any>(self, dependent: NodeHandle) -> Option<Rc<T>> {
        let cell = self.find(TypeId::of::<T>())?;
        cell.depend(dependent);

        cell.read().downcast::<T>().ok()
    }

    fn find(&self, type_id: TypeId) -> Option<&ProvideCell> {
        let mut node = self.head;

        while let Some(ptr) = node {
            // SAFETY: each chain pointer addresses a live node owned by an ancestor element. That owner
            // outlives every descendant that can hold this scope, and the node sits in its own allocation,
            // never borrowed `&mut` after it is linked, so this shared read is sound even while the owner is
            // being reconciled through `&mut`.
            let current = unsafe { ptr.as_ref() };

            if current.cell.type_id == type_id {
                return Some(&current.cell);
            }

            node = current.parent;
        }

        None
    }
}

impl ProvideNode {
    /// Creates a node holding `value`, not yet linked into any scope.
    pub fn new<V: Any>(value: V) -> Self {
        Self {
            cell: ProvideCell::new(value),
            parent: None,
        }
    }

    /// The scope rooted at this node: its value, then everything above it.
    ///
    /// The returned scope points into this node's allocation, so the owner must keep the node at a stable
    /// address (box it) for as long as any descendant can read through the scope.
    pub fn scope(&self) -> ProvideScope {
        ProvideScope {
            head: Some(NonNull::from(self)),
        }
    }

    /// Links this node above `parent`'s chain. Call once, when the owner mounts and the node's address is
    /// final.
    pub fn set_parent(&mut self, parent: ProvideScope) {
        self.parent = parent.head;
    }

    /// The current value, if it is of type `V`.
    pub fn current<V: Any>(&self) -> Option<Rc<V>> {
        self.cell.read().downcast::<V>().ok()
    }

    /// Replaces the held value with `value`, returning the readers to mark for a dependency-change rebuild.
    pub fn replace<V: Any>(&self, value: V) -> Vec<NodeHandle> {
        self.cell.replace(Rc::new(value))
    }
}
