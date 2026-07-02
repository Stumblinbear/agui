use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::ptr::NonNull;

mod any_render_object;
mod object;

pub use any_render_object::*;
pub use object::*;

/// Storage for an element's render object. The render object is interior-mutable, so a parent reaching it
/// during layout and an isolated relayout can both borrow it as `&mut` across the `&mut element` retags of
/// rebuilds without aliasing UB. Borrowed only during a layout or paint pass.
///
/// An element holds its render object in one of these and hands a parent a pointer to it with
/// [`render_object_ptr`](Self::render_object_ptr). Render-object code never sees the cell: it receives a plain
/// `&mut R`.
///
/// In debug it asserts no overlapping borrow; in release it is a bare [`UnsafeCell`], the same size as `R`.
pub struct RenderObjectCell<R: ?Sized> {
    inner: UnsafeCell<RenderObjectInner<R>>,
}

/// The interior of a [`RenderObjectCell`]: the render object plus, in debug, a borrow flag. The flag lives
/// inside the cell so it is reachable through the same `.get()`-derived pointer as the render object.
///
/// Opaque outside this crate: the type is nameable, so a protocol layer can unsize `R` to its erased `dyn`
/// form (see [`RenderObjectPtr::as_inner`]), but the fields are private, so the render object and the borrow
/// flag stay unreachable.
pub struct RenderObjectInner<R: ?Sized> {
    #[cfg(debug_assertions)]
    borrows: Cell<isize>,
    // Last field, so `RenderObjectInner<Concrete>` unsizes to `RenderObjectInner<dyn …>`.
    value: R,
}

impl<R> RenderObjectCell<R> {
    pub fn new(value: R) -> Self {
        Self {
            inner: UnsafeCell::new(RenderObjectInner {
                #[cfg(debug_assertions)]
                borrows: Cell::new(0),
                value,
            }),
        }
    }
}

impl<R: ?Sized> RenderObjectCell<R> {
    /// The render object, by exclusive reference.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner.get_mut().value
    }

    /// A pointer to the render object, taken via `.get()` so it carries interior-mutable provenance. A parent
    /// resolves one of these each pass and borrows it.
    pub fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        // SAFETY: `.get()` is the blessed interior-mutable derivation; the pointer is borrowed only at pass time.
        RenderObjectPtr(unsafe { NonNull::new_unchecked(self.inner.get()) })
    }
}

/// A pointer to a render object's [`RenderObjectCell`], resolved during a pass and borrowed to reach the
/// render object. An element produces one with `Element::render_object_ptr`; a parent resolves it fresh each
/// pass from the child's address.
pub struct RenderObjectPtr<R: ?Sized>(NonNull<RenderObjectInner<R>>);

impl RenderObjectPtr<()> {
    /// The placeholder for a render-less element (`Render = ()`): a shared per-thread empty cell, live for
    /// the thread's life. It may be borrowed like any other render object.
    pub fn unit() -> Self {
        thread_local! {
            static UNIT: UnsafeCell<RenderObjectInner<()>> = const {
                UnsafeCell::new(RenderObjectInner {
                    #[cfg(debug_assertions)]
                    borrows: Cell::new(0),
                    value: (),
                })
            };
        }

        // SAFETY: the pointer addresses this thread's placeholder cell, and render objects are only ever
        // reached from the thread that owns their tree.
        UNIT.with(|cell| RenderObjectPtr(unsafe { NonNull::new_unchecked(cell.get()) }))
    }
}

impl<R: ?Sized> RenderObjectPtr<R> {
    /// The pointer to the cell interior, for a protocol layer to unsize `R` to its erased `dyn` form and pair
    /// with [`from_inner`](Self::from_inner). The interior is opaque, so this hands out only the address, not
    /// access to the render object.
    pub fn as_inner(self) -> NonNull<RenderObjectInner<R>> {
        self.0
    }

    /// Rebuilds a pointer from a cell interior obtained from [`as_inner`](Self::as_inner), typically after
    /// unsizing `R` to an erased protocol type.
    ///
    /// # Safety
    /// `inner` must address a live render object cell whose stored value is an `R`.
    pub unsafe fn from_inner(inner: NonNull<RenderObjectInner<R>>) -> Self {
        RenderObjectPtr(inner)
    }

    /// Shared-borrows the render object for the chosen lifetime, registering a debug borrow released when the
    /// guard drops.
    ///
    /// # Safety
    /// Borrow only during a layout or paint pass, with no element hook on the call stack. The borrow must not
    /// outlive the cell, and no exclusive borrow may coexist with it. The caller chooses `'a`, so it must bound
    /// the guard to a reference it holds for no longer than the pass.
    pub unsafe fn borrow<'a>(self) -> RenderObjectRef<'a, R> {
        debug_mark_shared(self.0);
        RenderObjectRef {
            inner: self.0,
            _marker: PhantomData,
        }
    }

    /// Exclusively borrows the render object for the chosen lifetime, registering a debug borrow released when
    /// the guard drops.
    ///
    /// # Safety
    /// As [`borrow`](Self::borrow), and additionally no other borrow, shared or exclusive, may coexist.
    pub unsafe fn borrow_mut<'a>(self) -> RenderObjectMut<'a, R> {
        debug_mark_exclusive(self.0);
        RenderObjectMut {
            inner: self.0,
            _marker: PhantomData,
        }
    }

    /// Shared-borrows the render object as a bare reference, without registering a debug borrow.
    ///
    /// # Safety
    /// As [`borrow`](Self::borrow), but it records nothing, so the caller must rule out a coexisting exclusive
    /// borrow by other means, such as holding a `&self` whose lifetime the result is bound to.
    pub unsafe fn value_ref<'a>(self) -> &'a R {
        // SAFETY: the caller upholds the borrow contract above.
        unsafe { &(*self.0.as_ptr()).value }
    }
}

/// A scoped shared borrow of a render object, produced by [`RenderObjectPtr::borrow`].
pub struct RenderObjectRef<'a, R: ?Sized> {
    inner: NonNull<RenderObjectInner<R>>,
    _marker: PhantomData<&'a R>,
}

impl<R: ?Sized> std::ops::Deref for RenderObjectRef<'_, R> {
    type Target = R;

    fn deref(&self) -> &R {
        // SAFETY: the unsafe borrow that produced this guard established shared access for its lifetime.
        unsafe { &(*self.inner.as_ptr()).value }
    }
}

/// A scoped exclusive borrow of a render object, produced by [`RenderObjectPtr::borrow_mut`].
pub struct RenderObjectMut<'a, R: ?Sized> {
    inner: NonNull<RenderObjectInner<R>>,
    _marker: PhantomData<&'a mut R>,
}

impl<R: ?Sized> std::ops::Deref for RenderObjectMut<'_, R> {
    type Target = R;

    fn deref(&self) -> &R {
        // SAFETY: the unsafe borrow that produced this guard established exclusive access for its lifetime.
        unsafe { &(*self.inner.as_ptr()).value }
    }
}

impl<R: ?Sized> std::ops::DerefMut for RenderObjectMut<'_, R> {
    fn deref_mut(&mut self) -> &mut R {
        // SAFETY: as `deref`.
        unsafe { &mut (*self.inner.as_ptr()).value }
    }
}

#[cfg(debug_assertions)]
impl<R: ?Sized> Drop for RenderObjectRef<'_, R> {
    fn drop(&mut self) {
        // SAFETY: the cell outlives the borrow held during the pass.
        let borrows = unsafe { &(*self.inner.as_ptr()).borrows };
        borrows.set(borrows.get() - 1);
    }
}

#[cfg(debug_assertions)]
impl<R: ?Sized> Drop for RenderObjectMut<'_, R> {
    fn drop(&mut self) {
        // SAFETY: the cell outlives the borrow held during the pass.
        unsafe { (*self.inner.as_ptr()).borrows.set(0) };
    }
}

/// A parent's deferred handle to a child render object: the child node's address plus how to resolve its
/// render object pointer from it. Resolved fresh each pass, never dereferenced at mount, where the protected
/// `&mut` mount chain would forbid reaching the child through this separate address.
///
/// Each mounted child has exactly one handle, moved into the one place that owns it. A boundary hook that
/// needs another takes it through [`duplicate`](Self::duplicate).
pub struct MountedChild<R: ?Sized> {
    node: NonNull<()>,
    resolve: unsafe fn(NonNull<()>) -> RenderObjectPtr<R>,
}

impl<R: ?Sized> MountedChild<R> {
    /// Builds the handle from the child node's `address` and a `resolve` that projects it to the child's
    /// render object pointer.
    ///
    /// # Safety
    /// - `address` must be the mounted child node's address, valid for as long as the child is mounted.
    /// - `resolve` must project that address to the child's render object pointer.
    /// - This must be the child's only handle; further ones come only from [`duplicate`](Self::duplicate),
    ///   which carries the aliasing obligation.
    /// - Resolving the handle reads the child node, so no resolution may coexist with an exclusive borrow of
    ///   that node, such as one held during the node's own update walk.
    pub unsafe fn new(
        address: NonNull<()>,
        resolve: unsafe fn(NonNull<()>) -> RenderObjectPtr<R>,
    ) -> Self {
        Self {
            node: address,
            resolve,
        }
    }

    /// A second handle to the same child, for a boundary hook that re-lays or repaints the child from
    /// outside the render-object walk.
    ///
    /// # Safety
    /// - No borrow through the duplicate may overlap a borrow of the same render object through any other
    ///   handle.
    /// - The duplicate must not resolve after the child unmounts. Keep it where it drops before the child
    ///   can go away, such as a boundary registration its render object clears on a child swap.
    pub unsafe fn duplicate(&self) -> Self {
        Self {
            node: self.node,
            resolve: self.resolve,
        }
    }

    /// Resolves and shared-borrows the child render object, for a forwarding read during a pass.
    pub fn borrow(&self) -> RenderObjectRef<'_, R> {
        // SAFETY: shared borrows may overlap each other, and an exclusive borrow of the same render object
        // requires `&mut` on this sole handle (or a duplicate, whose holder rules the overlap out).
        unsafe { (self.resolve)(self.node).borrow() }
    }

    /// Resolves and exclusively borrows the child render object, for laying out, painting, or otherwise
    /// mutating it during a pass.
    pub fn borrow_mut(&mut self) -> RenderObjectMut<'_, R> {
        // SAFETY: `&mut self` on the child's sole handle is an exclusive path to its render object; a
        // duplicate's holder rules the overlap out.
        unsafe { (self.resolve)(self.node).borrow_mut() }
    }

    /// Resolves and shared-borrows the child render object as a bare reference bound to `&self`, for reading a
    /// protocol-specific field such as parent data.
    ///
    /// # Safety
    /// No exclusive borrow of the child render object may coexist with the returned reference. The result
    /// borrows `&self`, so the borrow checker forbids a [`borrow_mut`](Self::borrow_mut) of this handle while
    /// it is held.
    pub unsafe fn value_ref(&self) -> &R {
        // SAFETY: the caller upholds the no-exclusive-borrow contract; the read is bound to `&self`.
        unsafe { (self.resolve)(self.node).value_ref() }
    }
}

/// In debug, registers a shared borrow of the render object behind `inner`, panicking if it is currently
/// borrowed exclusively. The matching [`RenderObjectRef`] releases it on drop. A no-op in release.
#[cfg_attr(not(debug_assertions), inline(always))]
fn debug_mark_shared<R: ?Sized>(inner: NonNull<RenderObjectInner<R>>) {
    #[cfg(debug_assertions)]
    {
        // SAFETY: the cell is live and pinned for as long as the pass holds it.
        let borrows = unsafe { &(*inner.as_ptr()).borrows };
        assert!(
            borrows.get() >= 0,
            "render object reached for a shared borrow while it is borrowed mutably"
        );
        borrows.set(borrows.get() + 1);
    }
    let _ = inner;
}

/// In debug, registers an exclusive borrow of the render object behind `inner`, panicking if any borrow is
/// currently live. The matching [`RenderObjectMut`] releases it on drop. A no-op in release.
#[cfg_attr(not(debug_assertions), inline(always))]
fn debug_mark_exclusive<R: ?Sized>(inner: NonNull<RenderObjectInner<R>>) {
    #[cfg(debug_assertions)]
    {
        // SAFETY: the cell is live and pinned for as long as the pass holds it.
        let borrows = unsafe { &(*inner.as_ptr()).borrows };
        assert!(
            borrows.get() == 0,
            "render object reached for an exclusive borrow while it is already borrowed"
        );
        borrows.set(-1);
    }
    let _ = inner;
}
