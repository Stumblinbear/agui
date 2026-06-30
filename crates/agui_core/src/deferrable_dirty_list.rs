use std::cell::RefCell;
use std::rc::Rc;

/// A list of ids marked dirty, drained as a batch. Marks may also be deferred and applied on the next drain.
pub struct DeferrableDirtyList<K> {
    dirty: Vec<K>,
    /// Marks deferred through [`deferred_queue`](Self::deferred_queue), applied by the next
    /// [`take_dirty`](Self::take_dirty).
    deferred: Rc<RefCell<Vec<K>>>,
    /// Holds the ids from the last [`take_dirty`](Self::take_dirty) for reading through
    /// [`drained`](Self::drained), and is reused as the next drain's buffer.
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

    /// Applies any deferred marks, then moves all marked ids into the drain buffer, leaving the list clean,
    /// and returns the buffer to sort and dedup before reading it through [`drained`](Self::drained). The
    /// contents are unordered and may repeat an id. A mark made before the next call lands on the now-empty
    /// list, untouched by the drain in flight.
    pub fn take_dirty(&mut self) -> &mut Vec<K> {
        self.dirty.append(&mut self.deferred.borrow_mut());
        self.scratch.clear();
        std::mem::swap(&mut self.dirty, &mut self.scratch);
        &mut self.scratch
    }

    /// The ids moved aside by the last [`take_dirty`](Self::take_dirty), as left by any sort or dedup applied
    /// to them.
    pub fn drained(&self) -> &[K] {
        &self.scratch
    }

    /// The shared queue of deferred marks. Pushing an id into it marks that id on the next
    /// [`take_dirty`](Self::take_dirty).
    pub fn deferred_queue(&self) -> Rc<RefCell<Vec<K>>> {
        Rc::clone(&self.deferred)
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

        list.take_dirty();
        assert_eq!(list.drained(), [1, 3]);
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
        list.take_dirty();
        assert_eq!(list.drained(), [1]);

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

        list.take_dirty();
        assert_eq!(list.drained(), [1]);
    }
}
