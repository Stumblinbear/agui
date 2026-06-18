//! A data structure for incremental recomputation: a registry of opaque nodes plus, for each phase a
//! client defines, the set of nodes waiting on it. The reactor standardizes the bookkeeping and nothing
//! else: register a node, remove it, mark it stale for a phase, drain a phase shallowest-first.
//!
//! It does not drive the work. A client registers each unit as a [`Node`], keeps the [`NodeId`] it gets
//! back, and marks a node stale with [`Reactor::mark`]. To run a frame the client walks the phases it
//! cares about, draining each with [`Reactor::take_dirty`] and recomputing the nodes itself, free to
//! register and remove others as it goes. The reactor never recomputes a node or holds a borrow across
//! one, so it has no idea what a node is or what recomputing one does.

use slotmap::SlotMap;

slotmap::new_key_type! {
    /// Identifies a registered node within one [`Reactor`].
    pub struct NodeId;
}

/// A kind of recompute, which is also a phase a client drains in order. Distinct kinds occupy distinct
/// waiting sets, so a node can wait on several at once.
pub trait Reaction: Copy + Eq + 'static {
    /// This kind's slot among the waiting sets. Distinct kinds return distinct values below `REACTIONS`.
    fn index(self) -> u8;
}

impl Reaction for () {
    fn index(self) -> u8 {
        0
    }
}

/// A unit the reactor tracks. The reactor stores it opaquely and only ever reads its
/// [`depth`](Node::depth) to order a drain. Recomputing it is the client's job.
pub trait Node {
    /// The kinds of work this node can wait on.
    type Reaction: Reaction;

    /// Where this node sits in the client's nesting. [`take_dirty`](Reactor::take_dirty) hands back the
    /// shallowest first. The default leaves a drain in mark order.
    fn depth(&self) -> usize;
}

/// A registered node and the phases it is waiting on.
struct Cell<N> {
    node: N,

    /// Bit `kind.index()` is set while this node sits in that phase's waiting set, so a second mark for
    /// the same phase does not enroll it twice.
    enrolled: u8,
}

/// The registry. It owns nodes by id and keeps a waiting set per phase; a client registers, marks, and
/// drains through it but runs the recomputes itself.
///
/// This is the whole of what the build, layout, and paint trees share. Each registers its own [`Node`]
/// type and drives its own frame, and none of them are named here.
pub struct Reactor<N: Node, const REACTIONS_COUNT: usize> {
    cells: SlotMap<NodeId, Cell<N>>,

    /// One waiting set per phase, indexed by [`Reaction::index`].
    dirty: [Vec<NodeId>; REACTIONS_COUNT],
}

impl<N: Node, const REACTIONS_COUNT: usize> Default for Reactor<N, REACTIONS_COUNT> {
    fn default() -> Self {
        Self {
            cells: SlotMap::with_key(),
            dirty: [(); REACTIONS_COUNT].map(|()| Vec::new()),
        }
    }
}

impl<N: Node, const REACTIONS_COUNT: usize> Reactor<N, REACTIONS_COUNT> {
    /// Registers `node` and returns its [`NodeId`]. The node waits on nothing until the caller marks it.
    pub fn register(&mut self, node: N) -> NodeId {
        self.cells.insert(Cell { node, enrolled: 0 })
    }

    /// Removes the node and returns it, or `None` if it is already gone, taking it out of every phase set
    /// it was waiting in. Reusing the slot yields a fresh id, so an id kept against the old node never
    /// lands on a later one.
    pub fn remove(&mut self, node_id: NodeId) -> Option<N> {
        self.cells.remove(node_id).map(|cell| cell.node)
    }

    /// Borrows the node `node_id` names, or `None` if it is gone.
    pub fn get(&self, node_id: NodeId) -> Option<&N> {
        self.cells.get(node_id).map(|cell| &cell.node)
    }

    /// Borrows the node `node_id` names mutably, or `None` if it is gone.
    pub fn get_mut(&mut self, node_id: NodeId) -> Option<&mut N> {
        self.cells.get_mut(node_id).map(|cell| &mut cell.node)
    }

    /// Whether no phase has a node waiting.
    pub fn is_clean(&self) -> bool {
        self.dirty.iter().all(Vec::is_empty)
    }

    /// Enrolls the node `node_id` names to be recomputed for `reaction`. A node already waiting on that
    /// phase is not enrolled twice. An id whose node is gone enrolls nothing.
    pub fn mark(&mut self, node_id: NodeId, reaction: N::Reaction) {
        let slot = reaction.index();
        let bit = 1u8 << slot;

        let Some(cell) = self.cells.get_mut(node_id) else {
            return;
        };

        if cell.enrolled & bit != 0 {
            return;
        }

        cell.enrolled |= bit;
        self.dirty[slot as usize].push(node_id);
    }

    /// Drains the nodes waiting on `reaction` into `out`, shallowest depth first, leaving other phases
    /// untouched.
    pub fn take_dirty(&mut self, reaction: N::Reaction, out: &mut Vec<NodeId>) {
        let slot = reaction.index();
        let bit = 1u8 << slot;

        out.clear();
        std::mem::swap(&mut self.dirty[slot as usize], out);

        for &id in out.iter() {
            if let Some(cell) = self.cells.get_mut(id) {
                cell.enrolled &= !bit;
            }
        }

        let cells = &self.cells;
        out.sort_by_key(|&id| cells.get(id).map_or(0, |cell| cell.node.depth()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum Phase {
        First,
        Second,
    }

    impl Reaction for Phase {
        fn index(self) -> u8 {
            match self {
                Self::First => 0,
                Self::Second => 1,
            }
        }
    }

    struct Probe {
        depth: usize,
    }

    impl Node for Probe {
        type Reaction = Phase;

        fn depth(&self) -> usize {
            self.depth
        }
    }

    fn drained(reactor: &mut Reactor<Probe, 2>, phase: Phase) -> Vec<NodeId> {
        let mut out = Vec::new();
        reactor.take_dirty(phase, &mut out);
        out
    }

    #[test]
    fn take_dirty_returns_only_the_nodes_waiting_on_that_phase() {
        let mut reactor = Reactor::default();

        let a = reactor.register(Probe { depth: 0 });
        let b = reactor.register(Probe { depth: 0 });

        reactor.mark(a, Phase::First);
        reactor.mark(b, Phase::Second);

        assert_eq!(drained(&mut reactor, Phase::First), vec![a]);
        assert_eq!(drained(&mut reactor, Phase::Second), vec![b]);
        assert!(reactor.is_clean());
    }

    #[test]
    fn one_node_can_wait_on_several_phases() {
        let mut reactor = Reactor::default();

        let a = reactor.register(Probe { depth: 0 });
        reactor.mark(a, Phase::First);
        reactor.mark(a, Phase::Second);

        assert_eq!(drained(&mut reactor, Phase::First), vec![a]);
        assert!(!reactor.is_clean(), "the second phase is still waiting");
        assert_eq!(drained(&mut reactor, Phase::Second), vec![a]);
        assert!(reactor.is_clean());
    }

    #[test]
    fn take_dirty_hands_back_shallowest_first() {
        let mut reactor = Reactor::default();

        // Registered deep before shallow, to show the drain orders by depth, not registration.
        let deep = reactor.register(Probe { depth: 5 });
        let shallow = reactor.register(Probe { depth: 2 });

        reactor.mark(deep, Phase::First);
        reactor.mark(shallow, Phase::First);

        assert_eq!(drained(&mut reactor, Phase::First), vec![shallow, deep]);
    }

    #[test]
    fn a_phase_marked_twice_drains_once() {
        let mut reactor = Reactor::default();

        let a = reactor.register(Probe { depth: 0 });
        reactor.mark(a, Phase::First);
        reactor.mark(a, Phase::First);

        assert_eq!(drained(&mut reactor, Phase::First), vec![a]);
    }

    #[test]
    fn a_removed_node_is_skipped_at_the_next_drain() {
        let mut reactor = Reactor::default();

        let a = reactor.register(Probe { depth: 0 });
        let b = reactor.register(Probe { depth: 0 });
        reactor.mark(a, Phase::First);
        reactor.mark(b, Phase::First);

        reactor.remove(a);

        // Removal does not purge the id from the waiting set. The drain still lists it, but the reactor
        // reports it gone through `get`, so a caller skips it. Only the live node is reachable.
        let live: Vec<_> = drained(&mut reactor, Phase::First)
            .into_iter()
            .filter(|&id| reactor.get(id).is_some())
            .collect();

        assert_eq!(live, vec![b]);
        assert!(reactor.is_clean());
    }

    #[test]
    fn re_marking_after_a_drain_waits_again() {
        let mut reactor = Reactor::default();

        let a = reactor.register(Probe { depth: 0 });
        reactor.mark(a, Phase::First);
        let _ = drained(&mut reactor, Phase::First);

        // The drain cleared the enrollment, so the node can be marked afresh.
        reactor.mark(a, Phase::First);
        assert_eq!(drained(&mut reactor, Phase::First), vec![a]);
    }
}
