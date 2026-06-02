use std::cell::RefCell;

use fnv::FnvHashMap;

use crate::{
    context::{Dispatch, UpdateCtx},
    element::{Element, ElementNode},
    key::AnyKeyable,
    routing_id::RoutingId,
    widget::Widget,
};

/// The [`Element`] of a widget with a single child.
pub struct SingleChildElement<C> {
    pub child: ElementNode<C>,
}

impl<C: Element> Element for SingleChildElement<C> {}

impl<C: Element> SingleChildElement<C> {
    pub fn new<CV: Widget<Element = C>>(child: &CV, ctx: &mut UpdateCtx) -> Self {
        SingleChildElement {
            child: ElementNode::new(child.create_element(ctx)),
        }
    }

    pub fn update<CV: Widget<Element = C>>(&mut self, new: &CV, old: &CV, ctx: &mut UpdateCtx) {
        new.update(&mut self.child.element, old, ctx);
    }

    pub fn dispatch<CV: Widget<Element = C>>(
        &mut self,
        child: &CV,
        path: &[RoutingId],
        action: Dispatch,
    ) {
        child.dispatch(&mut self.child.element, path, action);
    }

    pub fn create_render_object<CV: Widget<Element = C>>(&self, child: &CV) -> CV::Render {
        child.create_render_object(&self.child.element)
    }

    pub fn update_render_object<CV: Widget<Element = C>>(
        &self,
        child: &CV,
        render_object: &mut CV::Render,
    ) {
        child.update_render_object(&self.child.element, render_object);
    }
}

/// The [`Element`] of a widget with a flat, keyed list of children.
pub struct MultiChildElement<C> {
    pub children: Vec<ElementNode<C>>,
}

impl<C: Element> Element for MultiChildElement<C> {}

impl<C: Element> MultiChildElement<C> {
    pub fn new<'v, CV>(len: usize, child_at: impl Fn(usize) -> &'v CV, ctx: &mut UpdateCtx) -> Self
    where
        CV: Widget<Element = C> + 'v,
    {
        MultiChildElement {
            children: (0..len)
                .map(|idx| {
                    ctx.with_routing_id(RoutingId::from_index(idx), |ctx| {
                        ElementNode::new(child_at(idx).create_element(ctx))
                    })
                })
                .collect(),
        }
    }

    pub fn dispatch<'v, CV>(
        &mut self,
        child_at: impl Fn(usize) -> &'v CV,
        path: &[RoutingId],
        action: Dispatch,
    ) where
        CV: Widget<Element = C> + 'v,
    {
        let Some((head, rest)) = path.split_first() else {
            unreachable!("dispatch path cannot be empty");
        };

        let idx = head.get() as usize;

        child_at(idx).dispatch(&mut self.children[idx].element, rest, action);
    }

    pub fn update<'v, CV>(
        &mut self,
        new_len: usize,
        new_at: impl Fn(usize) -> &'v CV,
        old_at: impl Fn(usize) -> &'v CV,
        ctx: &mut UpdateCtx,
    ) where
        CV: Widget<Element = C> + 'v,
    {
        // No children now: drop everything.
        if new_len == 0 {
            self.children.clear();
            return;
        }

        // No children before: materialize all of them.
        if self.children.is_empty() {
            self.children = (0..new_len)
                .map(|idx| {
                    ctx.with_routing_id(RoutingId::from_index(idx), |ctx| {
                        ElementNode::new(new_at(idx).create_element(ctx))
                    })
                })
                .collect();
            return;
        }

        let old_len = self.children.len();

        // Count the leading children that line up positionally (same type and key).
        let mut prefix = 0;
        while prefix < old_len
            && prefix < new_len
            && old_at(prefix).is_same_type(new_at(prefix))
            && old_at(prefix).key() == new_at(prefix).key()
        {
            prefix += 1;
        }

        // If we matched everything, update them all in-place and append/truncate the tail if needed.
        if prefix == old_len || prefix == new_len {
            for (index, child) in self.children.iter_mut().take(prefix).enumerate() {
                ctx.with_routing_id(RoutingId::from_index(index), |ctx| {
                    new_at(index).update(&mut child.element, old_at(index), ctx);
                });
            }

            if prefix == old_len {
                self.children.reserve(new_len - old_len);
                for index in prefix..new_len {
                    let node = ctx.with_routing_id(RoutingId::from_index(index), |ctx| {
                        ElementNode::new(new_at(index).create_element(ctx))
                    });
                    self.children.push(node);
                }
            } else {
                self.children.truncate(new_len);
            }

            return;
        }

        // We've got some remaining children that we need to reconcile. We do this through
        // dynamic dispatch so the process is emitted once instead of monomorphized per child type.
        let mut sink = ReconcileAdapter {
            children: &mut self.children,
            new_at,
            old_at,
            old_elements: Vec::new(),
        };

        drive_reconcile(&mut sink, prefix, old_len, new_len, ctx);
    }
}

// Scratch index reused across reconciles so a reorder does not reallocate the keyed map each
// frame. Keys are laundered to `'static`; `match_keyed_middle` clears the map before returning,
// so no laundered reference is ever observed afterwards.
thread_local! {
    #[allow(clippy::mutable_key_type)]
    static KEYED_SCRATCH: RefCell<FnvHashMap<&'static dyn AnyKeyable, usize>> =
        RefCell::new(FnvHashMap::default());
}

trait ReconcileSink {
    /// Whether the old child can be updated in place by the new child (same type and key).
    fn can_update(&self, old_index: usize, new_index: usize) -> bool;

    fn old_key(&self, index: usize) -> Option<&dyn AnyKeyable>;

    fn new_key(&self, index: usize) -> Option<&dyn AnyKeyable>;

    /// Move the old children into scratch and reserve the emptied child list for `new_len` results.
    fn begin(&mut self, new_len: usize);

    /// Reuse the old element at `old_index` for new slot `new_index`, updating it in place.
    ///
    /// Results are appended strictly left to right, so `new_index` is supplied only for the routing
    /// id; the implementation pushes rather than indexing.
    fn reuse(&mut self, new_index: usize, old_index: usize, ctx: &mut UpdateCtx);

    /// Build a fresh element for new slot `new_index`, appending it to the result list.
    fn create(&mut self, new_index: usize, ctx: &mut UpdateCtx);
}

struct ReconcileAdapter<'a, C, NewAt, OldAt> {
    children: &'a mut Vec<ElementNode<C>>,
    new_at: NewAt,
    old_at: OldAt,
    old_elements: Vec<Option<ElementNode<C>>>,
}

impl<'a, 'v, C, CV, NewAt, OldAt> ReconcileSink for ReconcileAdapter<'a, C, NewAt, OldAt>
where
    'v: 'a,
    C: Element,
    CV: Widget<Element = C> + 'v,
    NewAt: Fn(usize) -> &'v CV,
    OldAt: Fn(usize) -> &'v CV,
{
    fn can_update(&self, old_index: usize, new_index: usize) -> bool {
        let old = (self.old_at)(old_index);
        let new = (self.new_at)(new_index);
        old.is_same_type(new) && old.key() == new.key()
    }

    fn old_key(&self, index: usize) -> Option<&dyn AnyKeyable> {
        (self.old_at)(index).key()
    }

    fn new_key(&self, index: usize) -> Option<&dyn AnyKeyable> {
        (self.new_at)(index).key()
    }

    fn begin(&mut self, new_len: usize) {
        // Move the old children into scratch (consuming the old `children` allocation), then reuse
        // the now-empty `children` Vec as the result buffer: `drive_reconcile` fills result slots
        // strictly left to right, so `reuse`/`create` push in order rather than indexing. Old nodes
        // left untaken are orphans, dropped when `old_elements` falls out of scope.
        self.old_elements = std::mem::take(self.children)
            .into_iter()
            .map(Some)
            .collect();

        self.children.reserve(new_len);
    }

    fn reuse(&mut self, new_index: usize, old_index: usize, ctx: &mut UpdateCtx) {
        let mut node = self.old_elements[old_index]
            .take()
            .expect("reconcile reused an old child more than once");

        ctx.with_routing_id(RoutingId::from_index(new_index), |ctx| {
            (self.new_at)(new_index).update(&mut node.element, (self.old_at)(old_index), ctx);
        });

        debug_assert_eq!(self.children.len(), new_index, "result filled out of order");

        self.children.push(node);
    }

    fn create(&mut self, new_index: usize, ctx: &mut UpdateCtx) {
        let node = ctx.with_routing_id(RoutingId::from_index(new_index), |ctx| {
            ElementNode::new((self.new_at)(new_index).create_element(ctx))
        });

        debug_assert_eq!(self.children.len(), new_index, "result filled out of order");

        self.children.push(node);
    }
}

#[inline(never)]
fn drive_reconcile(
    sink: &mut dyn ReconcileSink,
    start_at: usize,
    old_len: usize,
    new_len: usize,
    ctx: &mut UpdateCtx,
) {
    sink.begin(new_len);

    // The leading children before `start_at` were already matched by the caller,
    // so we can reuse them directly rather than re-running the comparison.
    for index in 0..start_at {
        sink.reuse(index, index, ctx);
    }

    let new_top = start_at;
    let old_top = start_at;
    let mut new_bottom = new_len;
    let mut old_bottom = old_len;

    // Read up from the bottom until we find two children that don't match.
    while old_top < old_bottom
        && new_top < new_bottom
        && sink.can_update(old_bottom - 1, new_bottom - 1)
    {
        old_bottom -= 1;
        new_bottom -= 1;
    }

    // If we have some new children left
    if new_top < new_bottom {
        // And we also have some old children left, we can try to match them by key.
        if old_top < old_bottom {
            let matches = match_keyed_middle(&*sink, old_top..old_bottom, new_top..new_bottom);

            for (offset, matched) in matches.into_iter().enumerate() {
                let new_index = new_top + offset;

                match matched {
                    Some(old_index) => sink.reuse(new_index, old_index, ctx),
                    None => sink.create(new_index, ctx),
                }
            }
        } else {
            for new_index in new_top..new_bottom {
                sink.create(new_index, ctx);
            }
        }
    }

    // Finally, we re-use the children that were matched by the bottom scan.
    for offset in 0..(new_len - new_bottom) {
        sink.reuse(new_bottom + offset, old_bottom + offset, ctx);
    }
}

#[inline(never)]
fn match_keyed_middle(
    sink: &dyn ReconcileSink,
    old_range: std::ops::Range<usize>,
    new_range: std::ops::Range<usize>,
) -> Vec<Option<usize>> {
    /// # Safety
    ///
    /// The returned reference must not outlive `key`. [`match_keyed_middle`] clears the map before
    /// returning and never recurses while holding it, so laundered keys never escape that scope.
    unsafe fn launder<'a>(key: &'a dyn AnyKeyable) -> &'static dyn AnyKeyable {
        unsafe { std::mem::transmute::<&'a dyn AnyKeyable, &'static dyn AnyKeyable>(key) }
    }

    KEYED_SCRATCH.with_borrow_mut(|old_keyed| {
        old_keyed.clear();

        for index in old_range {
            if let Some(key) = sink.old_key(index) {
                old_keyed.insert(unsafe { launder(key) }, index);
            }
        }

        let matches = new_range
            .map(|index| {
                sink.new_key(index).and_then(|key| {
                    let key = unsafe { launder(key) };
                    old_keyed.remove(&key)
                })
            })
            .collect();

        old_keyed.clear();

        matches
    })
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::{
        context::{Dispatch, UpdateCtx},
        element::Element,
        key::AnyKeyable,
        provide::ProvideScope,
        render_object::RenderLeaf,
        routing_id::RoutingId,
        test_harness::TestTaskRunner,
        widget::Widget,
    };

    use super::{MultiChildElement, SingleChildElement};

    fn with_ctx<R>(f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
        let mut task_runner = TestTaskRunner::new();
        let mut path = Vec::new();
        let scope = ProvideScope::new();

        f(&mut UpdateCtx::new(
            &mut task_runner.scheduler(),
            &mut path,
            scope,
        ))
    }

    struct Probe {
        id: u32,
        key: Option<u32>,
        mounts: Rc<Cell<usize>>,
        updates: Rc<Cell<usize>>,
    }

    struct ProbeElement {
        id: u32,
        mounted_id: u32,
    }

    impl Element for ProbeElement {}

    impl Widget for Probe {
        type Element = ProbeElement;

        type Render = RenderLeaf;

        fn create_element(&self, _: &mut UpdateCtx) -> ProbeElement {
            self.mounts.set(self.mounts.get() + 1);
            ProbeElement {
                id: self.id,
                mounted_id: self.id,
            }
        }

        fn update(&self, element: &mut ProbeElement, _: &Self, _: &mut UpdateCtx) {
            self.updates.set(self.updates.get() + 1);
            element.id = self.id;
        }

        fn create_render_object(&self, _: &ProbeElement) -> Self::Render {
            Self::Render::default()
        }

        fn update_render_object(&self, _: &ProbeElement, _: &mut Self::Render) {}

        fn key(&self) -> Option<&dyn AnyKeyable> {
            self.key.as_ref().map(|k| k as &dyn AnyKeyable)
        }
    }

    fn probes(ids: &[u32], mounts: &Rc<Cell<usize>>, updates: &Rc<Cell<usize>>) -> Vec<Probe> {
        ids.iter()
            .map(|&id| Probe {
                id,
                key: None,
                mounts: Rc::clone(mounts),
                updates: Rc::clone(updates),
            })
            .collect()
    }

    /// Build keyed probes from `(id, key)` pairs.
    fn keyed_probes(
        items: &[(u32, u32)],
        mounts: &Rc<Cell<usize>>,
        updates: &Rc<Cell<usize>>,
    ) -> Vec<Probe> {
        items
            .iter()
            .map(|&(id, key)| Probe {
                id,
                key: Some(key),
                mounts: Rc::clone(mounts),
                updates: Rc::clone(updates),
            })
            .collect()
    }

    fn child_ids(element: &MultiChildElement<ProbeElement>) -> Vec<u32> {
        element.children.iter().map(|n| n.element.id).collect()
    }

    /// The mount-stamped identities in order, showing which element instance sits at each position.
    fn mounted_ids(element: &MultiChildElement<ProbeElement>) -> Vec<u32> {
        element
            .children
            .iter()
            .map(|n| n.element.mounted_id)
            .collect()
    }

    #[test]
    fn mount_materializes_each_child() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let widgets = probes(&[10, 20, 30], &m, &u);

        let element = with_ctx(|ctx| MultiChildElement::new(widgets.len(), |i| &widgets[i], ctx));

        assert_eq!(m.get(), 3);
        assert_eq!(u.get(), 0);
        assert_eq!(child_ids(&element), vec![10, 20, 30]);
    }

    #[test]
    fn update_same_length_reuses_each_child_in_place() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = probes(&[1, 2, 3], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));

        let new = probes(&[4, 5, 6], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(m.get(), 3, "no children remounted");
        assert_eq!(u.get(), 3, "each child reconciled in place");
        assert_eq!(child_ids(&element), vec![4, 5, 6]);
    }

    #[test]
    fn appending_only_mounts_the_new_tail() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = probes(&[1, 2], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));

        let new = probes(&[1, 2, 3], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(m.get(), 3, "two initial mounts plus one appended");
        assert_eq!(u.get(), 2, "the two retained children updated");
        assert_eq!(child_ids(&element), vec![1, 2, 3]);
    }

    #[test]
    fn truncating_drops_the_extra_tail() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = probes(&[1, 2, 3], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));

        let new = probes(&[1, 2], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(m.get(), 3, "nothing new mounted");
        assert_eq!(u.get(), 2, "the two survivors updated");
        assert_eq!(child_ids(&element), vec![1, 2]);
    }

    #[test]
    fn updating_to_empty_clears_children() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = probes(&[1, 2], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));

        let new = probes(&[], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert!(element.children.is_empty());
    }

    #[test]
    fn updating_from_empty_materializes_children() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = probes(&[], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));
        assert!(element.children.is_empty());

        let new = probes(&[7, 8], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(m.get(), 2, "fresh children are mounted");
        assert_eq!(u.get(), 0, "none are updated");
        assert_eq!(child_ids(&element), vec![7, 8]);
    }

    #[test]
    fn single_child_reuses_its_element_across_update() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = Probe {
            id: 1,
            key: None,
            mounts: Rc::clone(&m),
            updates: Rc::clone(&u),
        };
        let mut element = with_ctx(|ctx| SingleChildElement::new(&old, ctx));
        assert_eq!(m.get(), 1);
        assert_eq!(element.child.element.id, 1);

        let new = Probe {
            id: 9,
            key: None,
            mounts: Rc::clone(&m),
            updates: Rc::clone(&u),
        };
        with_ctx(|ctx| element.update(&new, &old, ctx));

        assert_eq!(m.get(), 1, "child reused, not remounted");
        assert_eq!(u.get(), 1);
        assert_eq!(element.child.element.id, 9);
    }

    #[test]
    fn keyed_children_swap_carries_state_with_the_key() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));

        // Two same-type keyed children: id 10 keyed 0, id 20 keyed 1.
        let old = keyed_probes(&[(10, 0), (20, 1)], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));
        assert_eq!(mounted_ids(&element), vec![10, 20]);

        // Reorder to [key 1, key 0] with fresh config ids, so we can see which element moved.
        let new = keyed_probes(&[(98, 1), (99, 0)], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(m.get(), 2, "both elements reused, neither remounted");
        // The element mounted as key 1 (mounted_id 20) followed its key to position 0; key 0
        // (mounted_id 10) to position 1. State moved with the key, not the position.
        assert_eq!(mounted_ids(&element), vec![20, 10]);
        // Config (new widget ids) lands in the new order.
        assert_eq!(child_ids(&element), vec![98, 99]);
    }

    #[test]
    fn keyed_reorder_reuses_drops_and_creates_together() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));

        // Keys 0,1,2 mounted as ids 10,20,30.
        let old = keyed_probes(&[(10, 0), (20, 1), (30, 2)], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));
        assert_eq!(mounted_ids(&element), vec![10, 20, 30]);

        // New order [key 2, key 5 (new), key 0]: key 2 and key 0 reuse, key 5 is created, key 1
        // has no new home and is dropped.
        let new = keyed_probes(&[(91, 2), (92, 5), (93, 0)], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(
            m.get(),
            4,
            "three initial mounts plus the one created child"
        );
        // Reused elements followed their keys; the created child carries its own mounted id; key 1
        // (mounted_id 20) is gone.
        assert_eq!(mounted_ids(&element), vec![30, 92, 10]);
        assert_eq!(child_ids(&element), vec![91, 92, 93]);
    }

    /// A keyed widget whose element is itself a [`MultiChildElement`], so nesting two of them lets a
    /// reorder of the outer list recurse into a reorder of an inner list.
    struct Group {
        key: u32,
        children: Vec<Probe>,
    }

    impl Widget for Group {
        type Element = MultiChildElement<ProbeElement>;

        type Render = RenderLeaf;

        fn create_element(&self, ctx: &mut UpdateCtx) -> Self::Element {
            MultiChildElement::new(self.children.len(), |i| &self.children[i], ctx)
        }

        fn update(&self, element: &mut Self::Element, old: &Self, ctx: &mut UpdateCtx) {
            element.update(
                self.children.len(),
                |i| &self.children[i],
                |i| &old.children[i],
                ctx,
            );
        }

        fn dispatch(&self, element: &mut Self::Element, path: &[RoutingId], action: Dispatch) {
            element.dispatch(|i| &self.children[i], path, action);
        }

        fn create_render_object(&self, _: &Self::Element) -> Self::Render {
            Self::Render::default()
        }

        fn update_render_object(&self, _: &Self::Element, _: &mut Self::Render) {}

        fn key(&self) -> Option<&dyn AnyKeyable> {
            Some(&self.key)
        }
    }

    #[test]
    fn nested_reorder_does_not_reentrantly_borrow_the_keyed_map() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let group = |key, items: &[(u32, u32)]| Group {
            key,
            children: keyed_probes(items, &m, &u),
        };

        // Two keyed groups, each a keyed child list.
        let old = [
            group(0, &[(100, 0), (101, 1)]),
            group(1, &[(200, 0), (201, 1)]),
        ];
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));
        assert_eq!(m.get(), 4, "four leaf probes mounted across the two groups");

        // Reorder the groups (key 1 first) AND reorder the probes inside each group. The outer
        // reorder recurses into each inner reorder while applying its plan; were the single-slot map
        // borrowed across that recursion this would panic with a double borrow.
        let new = [
            group(1, &[(210, 1), (211, 0)]),
            group(0, &[(110, 1), (111, 0)]),
        ];
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], |i| &old[i], ctx));

        assert_eq!(m.get(), 4, "everything reused by key, nothing remounted");

        // Each element instance followed its key down both levels: group key 1 to outer index 0
        // with its probes reordered key-1-first, group key 0 to index 1.
        let nested: Vec<Vec<u32>> = element
            .children
            .iter()
            .map(|g| {
                g.element
                    .children
                    .iter()
                    .map(|n| n.element.mounted_id)
                    .collect()
            })
            .collect();
        assert_eq!(nested, vec![vec![201, 200], vec![101, 100]]);
    }
}
