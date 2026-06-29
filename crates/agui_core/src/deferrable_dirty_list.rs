use std::cell::RefCell;
use std::rc::Rc;

/// A list of ids marked dirty, drained as a batch. Marks may also be deferred and applied on the next drain.
pub struct DeferrableDirtyList<K> {
    dirty: Vec<K>,
    /// Marks deferred through [`deferred_queue`](Self::deferred_queue), applied by
    /// [`drain_deferred`](Self::drain_deferred).
    deferred: Rc<RefCell<Vec<K>>>,
    /// A drained list retained for the next [`take_dirty`](Self::take_dirty) to reuse.
    scratch: Vec<K>,
}

impl<K> Default for DeferrableDirtyList<K> {
    fn default() -> Self {
        Self {
            dirty: Vec::new(),
            deferred: Rc::new(RefCell::new(Vec::new())),
            scratch: Vec::new(),
        }
    }
}

impl<K> DeferrableDirtyList<K> {
    /// Marks `id`. Returns `true` on the transition from clean to dirty.
    pub fn mark(&mut self, id: K) -> bool {
        let was_clean = self.dirty.is_empty();
        self.dirty.push(id);
        was_clean
    }

    /// Whether nothing is marked.
    pub fn is_clean(&self) -> bool {
        self.dirty.is_empty()
    }

    /// Drains the marked ids, leaving the list clean. The batch is unordered and may contain an id more than
    /// once.
    pub fn take_dirty(&mut self) -> Vec<K> {
        let mut out = std::mem::take(&mut self.scratch);
        out.clear();
        std::mem::swap(&mut self.dirty, &mut out);
        out
    }

    /// Hands a drained list back for the next [`take_dirty`](Self::take_dirty) to reuse.
    pub fn recycle(&mut self, mut drained: Vec<K>) {
        drained.clear();
        self.scratch = drained;
    }

    /// The shared queue of deferred marks. Pushing an id into it marks that id on the next
    /// [`drain_deferred`](Self::drain_deferred).
    pub fn deferred_queue(&self) -> Rc<RefCell<Vec<K>>> {
        Rc::clone(&self.deferred)
    }

    /// Applies the deferred marks, moving each onto the dirty list.
    pub fn drain_deferred(&mut self) {
        let queued: Vec<K> = self.deferred.borrow_mut().drain(..).collect();
        for id in queued {
            self.mark(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DeferrableDirtyList;

    #[test]
    fn drains_every_mark() {
        let mut list = DeferrableDirtyList::<u32>::default();
        list.mark(1);
        list.mark(3);

        assert_eq!(list.take_dirty(), vec![1, 3]);
    }

    #[test]
    fn mark_reports_only_the_clean_to_dirty_transition() {
        let mut list = DeferrableDirtyList::<u32>::default();

        assert!(list.mark(1), "first mark takes the list dirty");
        assert!(!list.mark(1), "a second mark does not");
        assert!(
            !list.mark(2),
            "nor does marking another while already dirty"
        );
    }

    #[test]
    fn draining_leaves_it_clean_so_a_later_mark_transitions() {
        let mut list = DeferrableDirtyList::<u32>::default();

        list.mark(1);
        let drained = list.take_dirty();
        assert_eq!(drained, vec![1]);
        list.recycle(drained);

        assert!(list.is_clean());
        assert!(list.mark(1), "clean again, so the mark transitions it");
    }

    #[test]
    fn deferred_marks_apply_on_drain() {
        let mut list = DeferrableDirtyList::<u32>::default();

        list.deferred_queue().borrow_mut().push(1);
        assert!(
            list.is_clean(),
            "a deferred mark does not enqueue until drained"
        );

        list.drain_deferred();
        assert_eq!(list.take_dirty(), vec![1]);
    }
}
