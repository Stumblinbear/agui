use std::any::Any;
use std::future::Future;
use std::ptr::NonNull;
use std::rc::Rc;

use agui_core::tree::{Cursor, NodeContainer, NodeHandle};

use crate::context::{BuildCtx, CreateCtx, TaskCtx};
use crate::element::Element;
use crate::pipeline::build_tree::{Build, BuildQueue, run};
use crate::pipeline::render_pipeline::{LayoutScope, PaintScope, RenderPipeline};
use crate::provide::ProvideScope;
use crate::render_object::node::{MountedChild, RenderObjectPtr};
use crate::scheduling::{TaskHandle, TaskScheduler};

/// The context passed to an element during a cursor-bearing lifecycle hook: mount, unmount, rebuild, or a
/// dependency change. It carries a cursor to edit the element's children, the values in scope, the dirty
/// set, and the scheduler the element spawns tasks on.
pub struct UpdateCtx<'a> {
    cursor: Cursor<'a, Build>,
    provide: ProvideScope,
    queue: &'a mut BuildQueue,
    pipeline: &'a RenderPipeline,
    scheduler: &'a mut dyn TaskScheduler,
}

impl<'a> UpdateCtx<'a> {
    /// Wraps a cursor positioned at an element with the scope it builds under, the dirty set, the render
    /// pipeline, and the task scheduler. The driver builds one to mount the root or to dispatch a rebuild to
    /// an element by handle.
    pub fn new(
        cursor: Cursor<'a, Build>,
        provide: ProvideScope,
        queue: &'a mut BuildQueue,
        pipeline: &'a RenderPipeline,
        scheduler: &'a mut dyn TaskScheduler,
    ) -> Self {
        Self {
            cursor,
            provide,
            queue,
            pipeline,
            scheduler,
        }
    }

    /// Marks `scope`'s relayout boundary for re-layout on the next frame, as a reconcile does when it changes
    /// a layout-affecting property of a render object.
    pub fn mark_needs_layout(&self, scope: LayoutScope) {
        self.pipeline.mark_needs_layout(scope);
    }

    /// Marks `scope`'s repaint boundary to be repainted on the next frame.
    pub fn mark_needs_paint(&self, scope: PaintScope) {
        self.pipeline.mark_needs_paint(scope);
    }

    /// Marks `scope`'s compositing bits for recomputation before its next repaint, and the boundary for
    /// repaint, as a reconcile does when it changes a render object's compositing need.
    pub fn mark_needs_compositing_bits_update(&self, scope: PaintScope) {
        self.pipeline.mark_needs_compositing_bits_update(scope);
    }

    /// Queues the element at `dependent` to rebuild on the next flush, running its dependency-change hook
    /// first. A [`Provide`] calls this for each reader of a value it changed.
    ///
    /// [`Provide`]: crate::provide::Provide
    pub(crate) fn mark_dependency_changed(&mut self, dependent: NodeHandle) {
        self.queue.mark_dependency_changed(dependent);
    }

    /// This element's handle.
    pub fn handle(&self) -> NodeHandle {
        self.cursor.handle()
    }

    /// Spawns a task tied to this element. `func` receives a [`TaskCtx`] it can use to post a message back to
    /// this element, delivered on the next event drain. Hold the returned [`TaskHandle`] for as long as the
    /// task should run; dropping it cancels the task.
    pub fn spawn<F, Fut>(&mut self, func: F) -> Result<TaskHandle, Box<dyn std::error::Error>>
    where
        F: FnOnce(TaskCtx) -> Fut + 'static,
        Fut: Future<Output = ()> + 'static,
    {
        let task_ctx = TaskCtx::new(self.scheduler.event_tx(), self.cursor.handle());
        self.scheduler.spawn(Box::pin(func(task_ctx)))
    }

    /// An owned scheduler handle that outlives this build, for spawning a task once the cursor is gone, such
    /// as from a layout-time builder.
    pub fn deferred_scheduler(&self) -> Box<dyn TaskScheduler> {
        self.scheduler.deferred()
    }

    /// Re-bases the cursor onto `this`, the element behind a heap indirection (a `Box<dyn AnyElement>`), so its
    /// inline children register against its real address. The boxed-element boundary calls this before
    /// forwarding a cursor-bearing hook.
    ///
    /// # Safety
    /// As [`Cursor::rebase`](agui_core::tree::Cursor::rebase): `this` is the positioned element's address,
    /// with whole-allocation provenance.
    pub(crate) unsafe fn rebase(&mut self, this: NonNull<()>) {
        // SAFETY: the caller upholds `Cursor::rebase`'s contract.
        unsafe { self.cursor.rebase(this) };
    }

    /// The nearest provided value of type `T` in scope, or `None`.
    pub fn get_provided<T: Any>(&self) -> Option<Rc<T>> {
        self.provide.get::<T>()
    }

    /// Runs `f` with the restricted [`BuildCtx`] for a widget's `build`, which composes a child widget but
    /// does not edit the tree (no cursor). The context lives only for the call.
    pub fn build<R>(&self, f: impl FnOnce(&mut BuildCtx) -> R) -> R {
        f(&mut BuildCtx::new(self.provide, self.cursor.handle()))
    }

    /// Runs `f` with a [`CreateCtx`] for grafting a fresh child during this reconcile: it carries the scope
    /// and the pipeline, so a boundary built here registers its render tree, but registers no element-tree
    /// node (mount does that). The context lives only for the call.
    pub fn inflate<R>(&self, f: impl FnOnce(&mut CreateCtx) -> R) -> R {
        f(&mut CreateCtx::new(self.provide, self.pipeline.clone()))
    }

    /// The values currently in scope.
    pub fn provide(&self) -> ProvideScope {
        self.provide
    }

    /// Runs `func` with `scope` in scope, restoring the previous scope afterward. A [`Provide`] extends the
    /// scope for its child this way, and an element reached by a direct rebuild re-enters the scope it
    /// captured at mount.
    ///
    /// [`Provide`]: crate::provide::Provide
    pub fn with_scope<R>(
        &mut self,
        scope: ProvideScope,
        func: impl FnOnce(&mut UpdateCtx<'_>) -> R,
    ) -> R {
        let previous = std::mem::replace(&mut self.provide, scope);
        let result = func(self);
        self.provide = previous;
        result
    }

    /// Mounts `child` under this element and hands back the [`MountedChild`] pointing at its render object, now
    /// that it is registered and pinned. A render-bearing element passes the result to its render object's
    /// `adopt_child`; a transparent element ignores it.
    ///
    /// # Safety
    /// `child` must be one of this element's own slots.
    pub unsafe fn mount<S: NodeContainer>(
        &mut self,
        child: &mut S,
    ) -> MountedChild<<S::Node as Element>::Render>
    where
        S::Node: Element,
    {
        // SAFETY: the caller guarantees `child` is this element's, register's precondition; `run::<S::Node>`
        // dispatches it, and its unmount deregisters it.
        let cursor = unsafe { self.cursor.register(child, run::<S::Node>) };

        // The child node's address, taken as a value (not dereferenced), so it cannot conflict with the
        // protected `child` borrow. The render object is resolved from it later, at pass time.
        let address = cursor.this();

        child.node_mut().mount(&mut UpdateCtx {
            cursor,
            provide: self.provide,
            queue: &mut *self.queue,
            pipeline: self.pipeline,
            scheduler: &mut *self.scheduler,
        });

        // SAFETY: `address` is the now-mounted, pinned child node, and `resolve_render_object::<S::Node>`
        // projects it to that child's render object pointer when a pass dereferences it.
        unsafe { MountedChild::new(address, resolve_render_object::<S::Node>) }
    }

    /// Unmounts `child` and removes it from the tree. The caller still owns `child` and drops it to free it.
    ///
    /// # Safety
    /// `child` must be one of this element's own slots.
    pub unsafe fn unmount<S: NodeContainer>(&mut self, child: &mut S)
    where
        S::Node: Element,
    {
        let provide = self.provide;
        let queue = &mut *self.queue;
        let pipeline = self.pipeline;
        let scheduler = &mut *self.scheduler;
        // SAFETY: the caller guarantees `child` is this element's, with_child's precondition.
        unsafe {
            self.cursor.with_child(child, move |node, cursor| {
                node.unmount(&mut UpdateCtx {
                    cursor,
                    provide,
                    queue,
                    pipeline,
                    scheduler,
                });
            });
        }
        self.cursor.deregister(child);
    }

    /// Reconciles an existing `child` in place: hands `func` the child and an [`UpdateCtx`] positioned at it.
    ///
    /// # Safety
    /// `child` must be one of this element's own slots.
    pub unsafe fn with_child<S: NodeContainer, R>(
        &mut self,
        child: &mut S,
        func: impl FnOnce(&mut S::Node, &mut UpdateCtx<'_>) -> R,
    ) -> R {
        let provide = self.provide;
        let queue = &mut *self.queue;
        let pipeline = self.pipeline;
        let scheduler = &mut *self.scheduler;
        // SAFETY: the caller guarantees `child` is this element's, with_child's precondition.
        unsafe {
            self.cursor.with_child(child, move |child_element, cursor| {
                func(
                    child_element,
                    &mut UpdateCtx {
                        cursor,
                        provide,
                        queue,
                        pipeline,
                        scheduler,
                    },
                )
            })
        }
    }
}

/// Resolves a mounted child node's address to its render object pointer. A [`MountedChild`] pairs this with
/// the address so a parent's `RenderNode` can resolve the render object fresh each pass.
///
/// # Safety
///
/// `address` must be a live mounted child node of type `E`, reached during a layout or paint pass with no
/// element hook on the call stack.
unsafe fn resolve_render_object<E: Element>(address: NonNull<()>) -> RenderObjectPtr<E::Render> {
    // SAFETY: the caller guarantees a live `E` at `address` and no `&mut element` on the stack, so this shared
    // view is sound; it reads only the element's render object pointer.
    unsafe { address.cast::<E>().as_ref() }.render_object_ptr()
}
