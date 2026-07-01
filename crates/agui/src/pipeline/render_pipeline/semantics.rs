use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use slotmap::{Key, SlotMap};

use super::{RenderPipeline, SemanticsBoundaryId, SemanticsScope};

use crate::pipeline::BoundaryContent;
use crate::semantics::{SemanticsTree, SemanticsTreeBuilder};

/// The semantics boundaries of one tree and their pending re-reads. Carries its own clean-to-dirty callback,
/// and the monotonic id source so a node minted on any walk stays unique across boundaries and walks.
pub(crate) struct SemanticsState {
    registry: RefCell<SlotMap<SemanticsBoundaryId, SemanticsBoundaryCell>>,

    dirty: RefCell<Vec<SemanticsBoundaryId>>,
    deferred: Rc<RefCell<Vec<SemanticsBoundaryId>>>,

    counter: Cell<u64>,

    notify: RefCell<Box<dyn Fn()>>,
}

impl SemanticsState {
    pub(super) fn new() -> Self {
        Self {
            registry: RefCell::new(SlotMap::with_key()),
            dirty: RefCell::new(Vec::new()),
            deferred: Rc::new(RefCell::new(Vec::new())),
            counter: Cell::new(0),
            notify: RefCell::new(Box::new(|| {})),
        }
    }

    /// Registers a semantics boundary, returning the handle that owns and unregisters it. A
    /// [`View`](crate::view::View) registers its root boundary this way; render objects under it mark it through
    /// a scope the handle hands out.
    pub(crate) fn register(self: &Rc<Self>, content: BoundaryContent) -> SemanticsBoundaryHandle {
        let id = self
            .registry
            .borrow_mut()
            .insert(SemanticsBoundaryCell { content });

        SemanticsBoundaryHandle {
            id,
            channel: Rc::downgrade(self),
        }
    }

    /// A deferred marker for `scope`'s boundary, for a render object to mark it from outside a pass.
    pub(crate) fn deferred_scope(&self, scope: SemanticsScope) -> DeferredSemanticsScope {
        DeferredSemanticsScope {
            id: scope.0,
            queue: Some(Rc::clone(&self.deferred)),
        }
    }

    /// Marks `id`'s semantics changed, firing the channel's callback on the clean-to-dirty transition. Ignores
    /// a mark for a boundary already gone.
    pub(crate) fn mark(&self, id: SemanticsBoundaryId) {
        if !self.registry.borrow().contains_key(id) {
            return;
        }

        let mut dirty = self.dirty.borrow_mut();
        let was_clean = dirty.is_empty();
        dirty.push(id);
        drop(dirty);

        if was_clean {
            (self.notify.borrow())();
        }
    }
}

struct SemanticsBoundaryCell {
    content: BoundaryContent,
}

impl RenderPipeline {
    /// Registers `f` to fire when a view's semantics change, so the driver re-reads them.
    pub fn on_needs_semantics_update(&self, f: Box<dyn Fn()>) {
        *self.semantics.notify.borrow_mut() = f;
    }

    /// Re-walks each semantics boundary marked since the last frame and hands its freshly built
    /// [`SemanticsTree`] to `update`, then re-arms so the next change fires the callback again.
    pub fn flush_semantics(&self, mut update: impl FnMut(SemanticsBoundaryId, SemanticsTree)) {
        let mut dirty = self.semantics.dirty.borrow_mut();
        dirty.extend(self.semantics.deferred.borrow_mut().drain(..));
        dirty.sort_unstable();
        dirty.dedup();

        let mut counter = self.semantics.counter.get();

        for &id in dirty.iter() {
            let content = match self.semantics.registry.borrow().get(id) {
                Some(boundary) => Rc::clone(&boundary.content),
                None => continue,
            };

            let mut builder = SemanticsTreeBuilder::new(&mut counter);
            content.borrow_mut().dyn_build_semantics(&mut builder);
            let tree = SemanticsTree::new(builder.finish());

            update(id, tree);
        }

        self.semantics.counter.set(counter);
        dirty.clear();
    }
}

/// The sole owner of a registered semantics boundary, held by whatever established it. Dropping it
/// unregisters the boundary. Hand descendants the [`scope`](Self::scope) to mark it, and mark the boundary
/// itself through [`mark_needs_semantics_update`](Self::mark_needs_semantics_update).
pub struct SemanticsBoundaryHandle {
    id: SemanticsBoundaryId,
    channel: Weak<SemanticsState>,
}

impl SemanticsBoundaryHandle {
    /// Marks this boundary's semantics changed, firing the pipeline's semantics callback so the driver
    /// re-reads this view.
    pub fn mark_needs_semantics_update(&self) {
        if let Some(channel) = self.channel.upgrade() {
            channel.mark(self.id);
        }
    }

    /// This boundary's scope, for the view to thread down its subtree so descendants can mark it.
    #[must_use]
    pub fn scope(&self) -> SemanticsScope {
        SemanticsScope(self.id)
    }

    /// A deferred marker for this boundary, captured by a descendant render object to request a re-read
    /// when its semantics later change.
    #[must_use]
    pub fn deferred_scope(&self) -> DeferredSemanticsScope {
        let queue = self
            .channel
            .upgrade()
            .map(|channel| Rc::clone(&channel.deferred));

        DeferredSemanticsScope { id: self.id, queue }
    }

    /// Runs `f` with the pipeline's semantics id counter, so a full walk through the view mints from the same
    /// source as the per-boundary flush. The counter is copied out and written back, so the pipeline is not
    /// borrowed while `f` runs.
    pub(crate) fn with_counter<R>(&self, f: impl FnOnce(&mut u64) -> R) -> R {
        let channel = self
            .channel
            .upgrade()
            .expect("the pipeline outlives the view");

        let mut counter = channel.counter.get();
        let result = f(&mut counter);
        channel.counter.set(counter);

        result
    }
}

impl Drop for SemanticsBoundaryHandle {
    fn drop(&mut self) {
        let Some(channel) = self.channel.upgrade() else {
            return;
        };

        // A pending mark for this id is left in the dirty list: the next flush drops it when it finds no cell.
        let removed = channel.registry.borrow_mut().remove(self.id);

        // Drop the removed cell only after the borrow is released: it holds the boundary's render object,
        // whose drop may re-borrow the registry to unregister nested boundaries.
        drop(removed);
    }
}

/// A captured route to mark a semantics boundary from outside a pipeline pass, such as a property setter or
/// an animation tick. The request is applied on the pipeline's next frame. A detached scope, or one whose
/// boundary is gone, marks nothing.
#[derive(Clone)]
pub struct DeferredSemanticsScope {
    id: SemanticsBoundaryId,
    queue: Option<Rc<RefCell<Vec<SemanticsBoundaryId>>>>,
}

impl Default for DeferredSemanticsScope {
    fn default() -> Self {
        Self::detached()
    }
}

impl DeferredSemanticsScope {
    /// A scope that marks no boundary.
    #[must_use]
    pub fn detached() -> Self {
        Self {
            id: SemanticsBoundaryId::null(),
            queue: None,
        }
    }

    /// Queues this boundary's semantics to be re-read on the pipeline's next frame.
    pub fn mark_needs_semantics_update(&self) {
        if let Some(queue) = &self.queue {
            queue.borrow_mut().push(self.id);
        }
    }
}
