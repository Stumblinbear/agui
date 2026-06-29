use std::cell::{Cell, UnsafeCell};
use std::marker::PhantomData;
use std::ptr::NonNull;

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

impl<R> RenderObjectPtr<R> {
    /// A placeholder for a render-less element (`Render = ()`), never dereferenced.
    pub fn dangling() -> Self {
        RenderObjectPtr(NonNull::dangling())
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
    /// - Resolving the handle reads the child node, so no later resolution may coexist with an exclusive
    ///   borrow of that node, such as one held during the node's own update walk.
    pub unsafe fn new(
        address: NonNull<()>,
        resolve: unsafe fn(NonNull<()>) -> RenderObjectPtr<R>,
    ) -> Self {
        Self {
            node: address,
            resolve,
        }
    }

    /// Resolves and shared-borrows the child render object, for a forwarding read during a pass. Call only
    /// during a pass, with no exclusive borrow of the child node live.
    pub fn borrow(&self) -> RenderObjectRef<'_, R> {
        // SAFETY: no exclusive borrow of the child node is live, so this shared borrow cannot alias one; the
        // guard is bound to `&self`, held only during the pass.
        unsafe { (self.resolve)(self.node).borrow() }
    }

    /// Resolves and exclusively borrows the child render object, for a boundary holding this handle to re-lay
    /// or repaint it. Call only during a pass, with no borrow of the child node live.
    pub fn borrow_mut(&self) -> RenderObjectMut<'_, R> {
        // SAFETY: no exclusive borrow of the child node is live, so resolving and borrowing its render object
        // cannot alias one; the guard is bound to `&self`, held only during the pass.
        unsafe { (self.resolve)(self.node).borrow_mut() }
    }

    /// Resolves and shared-borrows the child render object as a bare reference bound to `&self`, for reading a
    /// protocol-specific field such as parent data. Call only during a pass.
    ///
    /// # Safety
    /// No exclusive borrow of the child node may coexist with the returned reference. The result borrows
    /// `&self`, so the borrow checker forbids a [`borrow_mut`](Self::borrow_mut) of this handle while it is held.
    pub unsafe fn value_ref(&self) -> &R {
        // SAFETY: the caller upholds the no-exclusive-borrow contract; the read is bound to `&self`.
        unsafe { (self.resolve)(self.node).value_ref() }
    }
}

// The node address and resolver are plain data; copying the handle just copies them.
impl<R: ?Sized> Clone for MountedChild<R> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<R: ?Sized> Copy for MountedChild<R> {}

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
