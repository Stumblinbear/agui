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
                    ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| {
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

        child_at(idx).dispatch(&mut self.children[idx].element, rest, action)
    }

    pub fn update<'v, CV>(
        &mut self,
        new_len: usize,
        new_at: impl Fn(usize) -> &'v CV,
        old_len: usize,
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
                    ctx.with_routing_id(RoutingId::new(idx as u16), |ctx| {
                        ElementNode::new(new_at(idx).create_element(ctx))
                    })
                })
                .collect();
            return;
        }

        assert!(
            old_len == self.children.len(),
            "multi-child count mismatch with the previous widget"
        );

        let mut new_top = 0;
        let mut old_top = 0;
        let mut new_bottom = new_len - 1;
        let mut old_bottom = old_len - 1;

        // Typed nodes can't be zeroed in place, so shuffle them through `Option` slots.
        let mut old_elements: Vec<Option<ElementNode<C>>> = std::mem::take(&mut self.children)
            .into_iter()
            .map(Some)
            .collect();

        let mut new_elements: Vec<Option<ElementNode<C>>> = (0..new_len).map(|_| None).collect();

        // Reuse in place only at the same type and key; a key change falls to the keyed middle.
        let can_update = |old: &CV, new: &CV| old.is_same_type(new) && old.key() == new.key();

        // Update the top of the list while the leading children still line up.
        while old_top <= old_bottom && new_top <= new_bottom {
            let old_child = old_at(old_top);
            let new_child = new_at(new_top);

            if !can_update(old_child, new_child) {
                break;
            }

            let mut node = old_elements[old_top].take().unwrap();

            ctx.with_routing_id(RoutingId::new(new_top as u16), |ctx| {
                new_child.update(&mut node.element, old_child, ctx)
            });

            new_elements[new_top] = Some(node);

            new_top += 1;
            old_top += 1;
        }

        // Scan the bottom of the list, leaving those children in place for now.
        while old_top <= old_bottom && new_top <= new_bottom {
            if !can_update(old_at(old_bottom), new_at(new_bottom)) {
                break;
            }

            old_bottom -= 1;
            new_bottom -= 1;
        }

        // Index the keys of the old children left in the middle so they can be matched by key.
        let have_old_children = old_top <= old_bottom;

        #[allow(clippy::mutable_key_type)]
        let mut old_keyed_children = FnvHashMap::<&dyn AnyKeyable, usize>::default();

        while old_top <= old_bottom {
            if let Some(key) = old_at(old_top).key() {
                old_keyed_children.insert(key, old_top);
            }

            old_top += 1;
        }

        // Update the middle of the list, reusing keyed children where they match.
        while new_top <= new_bottom {
            let new_child = new_at(new_top);

            let existing = if have_old_children {
                new_child
                    .key()
                    .and_then(|key| old_keyed_children.remove(&key))
            } else {
                None
            };

            if let Some(existing_idx) = existing {
                let mut node = old_elements[existing_idx].take().unwrap();

                ctx.with_routing_id(RoutingId::new(new_top as u16), |ctx| {
                    new_child.update(&mut node.element, old_at(existing_idx), ctx)
                });

                new_elements[new_top] = Some(node);
            } else {
                new_elements[new_top] =
                    Some(ctx.with_routing_id(RoutingId::new(new_top as u16), |ctx| {
                        ElementNode::new(new_child.create_element(ctx))
                    }));
            }

            new_top += 1;
        }

        // We've scanned the whole list.
        assert_eq!(old_top, old_bottom + 1);
        assert_eq!(new_top, new_bottom + 1);
        assert_eq!(new_len - new_top, old_len - old_top);

        new_bottom = new_len - 1;
        old_bottom = old_len - 1;

        // Update the bottom of the list (the suffix the bottom scan left untouched).
        while old_top <= old_bottom && new_top <= new_bottom {
            let mut node = old_elements[old_top].take().unwrap();

            ctx.with_routing_id(RoutingId::new(new_top as u16), |ctx| {
                new_at(new_top).update(&mut node.element, old_at(old_top), ctx)
            });

            new_elements[new_top] = Some(node);

            new_top += 1;
            old_top += 1;
        }

        self.children = new_elements
            .into_iter()
            .map(|node| node.expect("multi-child reconciliation left an unfilled child slot"))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc, sync::mpsc};

    use crate::{
        context::UpdateCtx, driver::Driver, element::Element, key::AnyKeyable,
        provide::ProvideScope, render_object::RenderLeaf, test_harness::NoopTestDriver,
        widget::Widget,
    };

    use super::{MultiChildElement, SingleChildElement};

    fn with_ctx<R>(f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
        let driver: Rc<dyn Driver> = Rc::new(NoopTestDriver);
        let (tx, _rx) = mpsc::channel();
        let mut path = Vec::new();
        let scope = ProvideScope::new();

        f(&mut UpdateCtx::new(&driver, &tx, &mut path, &scope))
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
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], old.len(), |i| &old[i], ctx));

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
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], old.len(), |i| &old[i], ctx));

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
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], old.len(), |i| &old[i], ctx));

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
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], old.len(), |i| &old[i], ctx));

        assert!(element.children.is_empty());
    }

    #[test]
    fn updating_from_empty_materializes_children() {
        let (m, u) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(0)));
        let old = probes(&[], &m, &u);
        let mut element = with_ctx(|ctx| MultiChildElement::new(old.len(), |i| &old[i], ctx));
        assert!(element.children.is_empty());

        let new = probes(&[7, 8], &m, &u);
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], old.len(), |i| &old[i], ctx));

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
        with_ctx(|ctx| element.update(new.len(), |i| &new[i], old.len(), |i| &old[i], ctx));

        assert_eq!(m.get(), 2, "both elements reused, neither remounted");
        // The element mounted as key 1 (mounted_id 20) followed its key to position 0; key 0
        // (mounted_id 10) to position 1. State moved with the key, not the position.
        assert_eq!(mounted_ids(&element), vec![20, 10]);
        // Config (new widget ids) lands in the new order.
        assert_eq!(child_ids(&element), vec![98, 99]);
    }
}
