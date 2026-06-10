use std::{any::TypeId, marker::PhantomData, ops::Range};

use fnv::FnvHashMap;

use crate::{
    context::{Dispatch, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, RoutingId, node::ElementNode},
    key::AnyKeyable,
    render_object::{MultiChildRenderObject, SingleChildRenderObject, node::RenderNode},
    widget::Widget,
};

/// The [`Element`] of a widget with a single child, threading the widget's render `R` to the child.
pub struct SingleChildElement<C, R: ?Sized> {
    pub child: ElementNode<C>,
    _render: PhantomData<fn() -> R>,
}

impl<C, R> Element for SingleChildElement<C, R>
where
    C: Element,
    R: SingleChildRenderObject<Child = C::Render>,
{
    type Render = R;

    fn dispatch(&mut self, render: &mut R, path: &[RoutingId], action: Dispatch) {
        let child = &mut self.child.element;
        render.with_child_mut(|child_render| child.dispatch(child_render, path, action));
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.element.describe(d))
            .finish()
    }
}

impl<C: Element, R: ?Sized> SingleChildElement<C, R> {
    /// Builds the element and the child's render object from `child`.
    pub fn new<CV: Widget<Element = C>>(child: CV, ctx: &mut UpdateCtx) -> (Self, CV::Render) {
        let (element, render) = child.create(ctx);

        (
            SingleChildElement {
                child: ElementNode::new(element),
                _render: PhantomData,
            },
            render,
        )
    }

    /// Reconciles the child element and `child_render` against `child`.
    pub fn update<CV: Widget<Element = C>>(
        &mut self,
        child: CV,
        child_render: &mut CV::Render,
        ctx: &mut UpdateCtx,
    ) {
        child.update(&mut self.child.element, child_render, ctx);
    }
}

/// The [`Element`] of a widget with a flat, keyed list of children, threading the widget's render `R`
/// to each child.
pub struct MultiChildElement<C, R: ?Sized> {
    children: Vec<KeyedNode<C>>,
    _render: PhantomData<fn() -> R>,
}

/// A child element paired with the type and key its widget reported at build.
struct KeyedNode<C> {
    node: ElementNode<C>,
    type_id: TypeId,
    key: Option<Box<dyn AnyKeyable>>,
}

impl<C, R> Element for MultiChildElement<C, R>
where
    C: Element,
    R: MultiChildRenderObject<Child = C::Render>,
{
    type Render = R;

    fn dispatch(&mut self, render: &mut R, path: &[RoutingId], action: Dispatch) {
        let Some((head, rest)) = path.split_first() else {
            // I'm not certain this is actually unreachable, but I can't prove it.
            unreachable!("multi-child element addresses one of its children");
        };

        let index = head.get() as usize;

        render.with_child_mut(index, |child_render| {
            self.children[index]
                .node
                .element
                .dispatch(child_render, rest, action);
        });
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        let mut builder = d.node_for::<Self>();

        for keyed in &self.children {
            builder = builder.child(|d| keyed.node.element.describe(d));
        }

        builder.finish()
    }
}

impl<C: Element, R: ?Sized> MultiChildElement<C, R> {
    /// Builds an element per child and installs each child's render object into `render`.
    pub fn new<CV>(
        children: Vec<CV>,
        render: &mut impl MultiChildRenderObject<Child = CV::Render>,
        ctx: &mut UpdateCtx,
    ) -> Self
    where
        CV: Widget<Element = C> + 'static,
    {
        let mut nodes = Vec::with_capacity(children.len());
        let mut render_children = Vec::with_capacity(children.len());

        for (idx, child) in children.into_iter().enumerate() {
            let type_id = child.widget_type_id();
            let key = child.key().map(AnyKeyable::dyn_clone);

            let (element, render_object) =
                ctx.with_routing_id(RoutingId::from_index(idx), |ctx| child.create(ctx));

            nodes.push(KeyedNode {
                node: ElementNode::new(element),
                type_id,
                key,
            });
            render_children.push(RenderNode::new(render_object));
        }

        render.set_children(render_children);

        Self {
            children: nodes,
            _render: PhantomData,
        }
    }

    /// Reconciles the child elements and `render`'s child render objects against `children`.
    pub fn update<CV>(
        &mut self,
        children: Vec<CV>,
        render: &mut impl MultiChildRenderObject<Child = CV::Render>,
        ctx: &mut UpdateCtx,
    ) where
        CV: Widget<Element = C> + 'static,
    {
        let old_render = render.take_children();
        let old = std::mem::take(&mut self.children);

        let (nodes, render_children) = reconcile::<C, CV>(old, old_render, children, ctx);

        self.children = nodes;
        render.set_children(render_children);
    }
}

/// Reconciles `new` against the old child elements and their render objects, returning the new
/// element list and render-object list in matching order. Old children left unmatched are dropped.
fn reconcile<C, CV>(
    old: Vec<KeyedNode<C>>,
    old_render: Vec<RenderNode<CV::Render>>,
    new: Vec<CV>,
    ctx: &mut UpdateCtx,
) -> (Vec<KeyedNode<C>>, Vec<RenderNode<CV::Render>>)
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    let new_len = new.len();
    let old_len = old.len();

    let mut out_nodes = Vec::with_capacity(new_len);
    let mut out_render = Vec::with_capacity(new_len);

    // No children now: drop everything.
    if new_len == 0 {
        return (out_nodes, out_render);
    }

    // No children before: materialize all of them.
    if old_len == 0 {
        for (index, child) in new.into_iter().enumerate() {
            create(&mut out_nodes, &mut out_render, index, child, ctx);
        }

        return (out_nodes, out_render);
    }

    let mut old_slots: Vec<Slot<C, CV::Render>> =
        old.into_iter().zip(old_render).map(Some).collect();
    let mut new_slots: Vec<Option<CV>> = new.into_iter().map(Some).collect();

    // Leading children that line up positionally.
    let mut prefix = 0;
    while prefix < old_len && prefix < new_len && can_update(&old_slots, &new_slots, prefix, prefix)
    {
        prefix += 1;
    }

    // Trailing children that line up from the bottom.
    let mut new_bottom = new_len;
    let mut old_bottom = old_len;
    while prefix < old_bottom
        && prefix < new_bottom
        && can_update(&old_slots, &new_slots, old_bottom - 1, new_bottom - 1)
    {
        old_bottom -= 1;
        new_bottom -= 1;
    }

    for index in 0..prefix {
        reuse(
            &mut out_nodes,
            &mut out_render,
            &mut old_slots,
            &mut new_slots,
            index,
            index,
            ctx,
        );
    }

    // The middle is matched by key; an unmatched new slot is created, an unmatched old one dropped.
    if prefix < new_bottom {
        if prefix < old_bottom {
            let plan = match_keyed_middle(
                &old_slots,
                &new_slots,
                prefix..old_bottom,
                prefix..new_bottom,
            );

            for (offset, matched) in plan.into_iter().enumerate() {
                let new_index = prefix + offset;

                if let Some(old_index) = matched {
                    reuse(
                        &mut out_nodes,
                        &mut out_render,
                        &mut old_slots,
                        &mut new_slots,
                        new_index,
                        old_index,
                        ctx,
                    );
                } else {
                    let child = new_slots[new_index].take().expect("new slot taken twice");
                    create(&mut out_nodes, &mut out_render, new_index, child, ctx);
                }
            }
        } else {
            for (offset, slot) in new_slots[prefix..new_bottom].iter_mut().enumerate() {
                let child = slot.take().expect("new slot taken twice");
                create(&mut out_nodes, &mut out_render, prefix + offset, child, ctx);
            }
        }
    }

    for offset in 0..(new_len - new_bottom) {
        reuse(
            &mut out_nodes,
            &mut out_render,
            &mut old_slots,
            &mut new_slots,
            new_bottom + offset,
            old_bottom + offset,
            ctx,
        );
    }

    (out_nodes, out_render)
}

type Slot<C, R> = Option<(KeyedNode<C>, RenderNode<R>)>;

type OldSlots<C, R> = [Slot<C, R>];

/// Whether the old child at `old_index` can be reconciled in place by the new child at `new_index`.
fn can_update<C, CV>(
    old_slots: &OldSlots<C, CV::Render>,
    new_slots: &[Option<CV>],
    old_index: usize,
    new_index: usize,
) -> bool
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    let keyed = &old_slots[old_index]
        .as_ref()
        .expect("compared a taken slot")
        .0;
    let child = new_slots[new_index]
        .as_ref()
        .expect("compared a taken slot");

    keyed.type_id == child.widget_type_id() && key_eq(child.key(), keyed.key.as_deref())
}

/// Reuses the old child at `old_index` for the new slot at `new_index`, reconciling it in place and
/// appending it to the result lists.
fn reuse<C, CV>(
    out_nodes: &mut Vec<KeyedNode<C>>,
    out_render: &mut Vec<RenderNode<CV::Render>>,
    old_slots: &mut OldSlots<C, CV::Render>,
    new_slots: &mut [Option<CV>],
    new_index: usize,
    old_index: usize,
    ctx: &mut UpdateCtx,
) where
    C: Element,
    CV: Widget<Element = C>,
{
    let (mut keyed, mut render) = old_slots[old_index].take().expect("reused a slot twice");
    let child = new_slots[new_index].take().expect("consumed a slot twice");

    keyed.key = child.key().map(AnyKeyable::dyn_clone);

    ctx.with_routing_id(RoutingId::from_index(new_index), |ctx| {
        child.update(&mut keyed.node.element, &mut render.object, ctx);
    });

    out_nodes.push(keyed);
    out_render.push(render);
}

/// Builds a fresh element and render object for the new child at `new_index`, appending both to the
/// result lists.
fn create<C, CV>(
    out_nodes: &mut Vec<KeyedNode<C>>,
    out_render: &mut Vec<RenderNode<CV::Render>>,
    new_index: usize,
    child: CV,
    ctx: &mut UpdateCtx,
) where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    let type_id = child.widget_type_id();
    let key = child.key().map(AnyKeyable::dyn_clone);

    let (element, render_object) =
        ctx.with_routing_id(RoutingId::from_index(new_index), |ctx| child.create(ctx));

    out_nodes.push(KeyedNode {
        node: ElementNode::new(element),
        type_id,
        key,
    });
    out_render.push(RenderNode::new(render_object));
}

/// For each keyed new child in `new_range`, the old index in `old_range` it reuses by key, or `None`
/// to create one.
fn match_keyed_middle<C, CV>(
    old_slots: &OldSlots<C, CV::Render>,
    new_slots: &[Option<CV>],
    old_range: Range<usize>,
    new_range: Range<usize>,
) -> Vec<Option<usize>>
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    #[allow(clippy::mutable_key_type)]
    let mut old_keyed: FnvHashMap<&dyn AnyKeyable, usize> = FnvHashMap::default();

    for index in old_range {
        if let Some((keyed, _)) = old_slots[index].as_ref()
            && let Some(key) = keyed.key.as_deref()
        {
            old_keyed.insert(key, index);
        }
    }

    new_range
        .map(|index| {
            new_slots[index]
                .as_ref()
                .and_then(|child| child.key())
                .and_then(|key| old_keyed.remove(&key))
        })
        .collect()
}

/// Whether two optional keys are equal, an absent key matching only another absent key.
fn key_eq(a: Option<&dyn AnyKeyable>, b: Option<&dyn AnyKeyable>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b,
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use crate::{
        context::{MountCtx, UpdateCtx},
        element::Element,
        key::AnyKeyable,
        render_object::RenderObject,
        test_fixtures::MultiChildRenderList,
        test_harness::with_ctx,
        widget::Widget,
    };

    use super::{MultiChildElement, SingleChildElement};

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

    /// The render object of a [`Probe`], stamped with its mount-time id so a reorder can be checked
    /// against the render tree as well as the element tree.
    struct ProbeRender {
        mounted_id: u32,
    }

    impl Element for ProbeElement {
        type Render = ProbeRender;
    }

    impl RenderObject for ProbeRender {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl Widget for Probe {
        type Element = ProbeElement;

        type Render = ProbeRender;

        fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            self.mounts.set(self.mounts.get() + 1);

            (
                ProbeElement {
                    id: self.id,
                    mounted_id: self.id,
                },
                ProbeRender {
                    mounted_id: self.id,
                },
            )
        }

        fn update(self, element: &mut Self::Element, _: &mut Self::Render, _: &mut UpdateCtx) {
            self.updates.set(self.updates.get() + 1);
            element.id = self.id;
        }

        fn key(&self) -> Option<&dyn AnyKeyable> {
            self.key.as_ref().map(|k| k as &dyn AnyKeyable)
        }
    }

    fn counter() -> Rc<Cell<usize>> {
        Rc::new(Cell::new(0))
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

    /// An empty render container for a list of probe children.
    fn render_list() -> MultiChildRenderList<ProbeRender> {
        MultiChildRenderList {
            children: Vec::new(),
        }
    }

    fn child_ids(
        element: &MultiChildElement<ProbeElement, MultiChildRenderList<ProbeRender>>,
    ) -> Vec<u32> {
        element.children.iter().map(|n| n.node.element.id).collect()
    }

    /// The mount-stamped identities in order, showing which element instance sits at each position.
    fn mounted_ids(
        element: &MultiChildElement<ProbeElement, MultiChildRenderList<ProbeRender>>,
    ) -> Vec<u32> {
        element
            .children
            .iter()
            .map(|n| n.node.element.mounted_id)
            .collect()
    }

    /// The mount-stamped identities of the render children, to check render/element lockstep.
    fn render_mounted_ids(render: &MultiChildRenderList<ProbeRender>) -> Vec<u32> {
        render
            .children
            .iter()
            .map(|n| n.object.mounted_id)
            .collect()
    }

    #[test]
    fn mount_materializes_each_child() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();
        let element =
            with_ctx(|ctx| MultiChildElement::new(probes(&[10, 20, 30], &m, &u), &mut render, ctx));

        assert_eq!(m.get(), 3);
        assert_eq!(u.get(), 0);
        assert_eq!(child_ids(&element), vec![10, 20, 30]);
        assert_eq!(render_mounted_ids(&render), vec![10, 20, 30]);
    }

    #[test]
    fn update_same_length_reuses_each_child_in_place() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();
        let mut element =
            with_ctx(|ctx| MultiChildElement::new(probes(&[1, 2, 3], &m, &u), &mut render, ctx));

        with_ctx(|ctx| element.update(probes(&[4, 5, 6], &m, &u), &mut render, ctx));

        assert_eq!(m.get(), 3, "no children remounted");
        assert_eq!(u.get(), 3, "each child reconciled in place");
        assert_eq!(child_ids(&element), vec![4, 5, 6]);
    }

    #[test]
    fn appending_only_mounts_the_new_tail() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();
        let mut element =
            with_ctx(|ctx| MultiChildElement::new(probes(&[1, 2], &m, &u), &mut render, ctx));

        with_ctx(|ctx| element.update(probes(&[1, 2, 3], &m, &u), &mut render, ctx));

        assert_eq!(m.get(), 3, "two initial mounts plus one appended");
        assert_eq!(u.get(), 2, "the two retained children updated");
        assert_eq!(child_ids(&element), vec![1, 2, 3]);
        assert_eq!(render_mounted_ids(&render), vec![1, 2, 3]);
    }

    #[test]
    fn truncating_drops_the_extra_tail() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();
        let mut element =
            with_ctx(|ctx| MultiChildElement::new(probes(&[1, 2, 3], &m, &u), &mut render, ctx));

        with_ctx(|ctx| element.update(probes(&[1, 2], &m, &u), &mut render, ctx));

        assert_eq!(m.get(), 3, "nothing new mounted");
        assert_eq!(u.get(), 2, "the two survivors updated");
        assert_eq!(child_ids(&element), vec![1, 2]);
        assert_eq!(render_mounted_ids(&render), vec![1, 2]);
    }

    #[test]
    fn updating_to_empty_clears_children() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();
        let mut element =
            with_ctx(|ctx| MultiChildElement::new(probes(&[1, 2], &m, &u), &mut render, ctx));

        with_ctx(|ctx| element.update(probes(&[], &m, &u), &mut render, ctx));

        assert!(child_ids(&element).is_empty());
        assert!(render.children.is_empty());
    }

    #[test]
    fn updating_from_empty_materializes_children() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();
        let mut element =
            with_ctx(|ctx| MultiChildElement::new(probes(&[], &m, &u), &mut render, ctx));
        assert!(child_ids(&element).is_empty());

        with_ctx(|ctx| element.update(probes(&[7, 8], &m, &u), &mut render, ctx));

        assert_eq!(m.get(), 2, "fresh children are mounted");
        assert_eq!(u.get(), 0, "none are updated");
        assert_eq!(child_ids(&element), vec![7, 8]);
        assert_eq!(render_mounted_ids(&render), vec![7, 8]);
    }

    #[test]
    fn single_child_reuses_its_element_across_update() {
        let (m, u) = (counter(), counter());
        let (mut element, mut render): (
            SingleChildElement<ProbeElement, ProbeRender>,
            ProbeRender,
        ) = with_ctx(|ctx| {
            SingleChildElement::new(
                Probe {
                    id: 1,
                    key: None,
                    mounts: Rc::clone(&m),
                    updates: Rc::clone(&u),
                },
                ctx,
            )
        });
        assert_eq!(m.get(), 1);
        assert_eq!(element.child.element.id, 1);

        with_ctx(|ctx| {
            element.update(
                Probe {
                    id: 9,
                    key: None,
                    mounts: Rc::clone(&m),
                    updates: Rc::clone(&u),
                },
                &mut render,
                ctx,
            );
        });

        assert_eq!(m.get(), 1, "child reused, not remounted");
        assert_eq!(u.get(), 1);
        assert_eq!(element.child.element.id, 9);
    }

    #[test]
    fn keyed_children_swap_carries_state_with_the_key() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();

        // Two same-type keyed children: id 10 keyed 0, id 20 keyed 1.
        let mut element = with_ctx(|ctx| {
            MultiChildElement::new(keyed_probes(&[(10, 0), (20, 1)], &m, &u), &mut render, ctx)
        });
        assert_eq!(mounted_ids(&element), vec![10, 20]);
        assert_eq!(render_mounted_ids(&render), vec![10, 20]);

        // Reorder to [key 1, key 0] with fresh config ids, so we can see which element moved.
        with_ctx(|ctx| element.update(keyed_probes(&[(98, 1), (99, 0)], &m, &u), &mut render, ctx));

        assert_eq!(m.get(), 2, "both elements reused, neither remounted");
        // State moved with the key: key 1 (mounted_id 20) to position 0, key 0 (mounted_id 10) to 1.
        assert_eq!(mounted_ids(&element), vec![20, 10]);
        // Config (new widget ids) lands in the new order.
        assert_eq!(child_ids(&element), vec![98, 99]);
        // The render children reordered in lockstep with the elements.
        assert_eq!(render_mounted_ids(&render), vec![20, 10]);
    }

    #[test]
    fn keyed_reorder_reuses_drops_and_creates_together() {
        let (m, u) = (counter(), counter());
        let mut render = render_list();

        // Keys 0,1,2 mounted as ids 10,20,30.
        let mut element = with_ctx(|ctx| {
            MultiChildElement::new(
                keyed_probes(&[(10, 0), (20, 1), (30, 2)], &m, &u),
                &mut render,
                ctx,
            )
        });
        assert_eq!(mounted_ids(&element), vec![10, 20, 30]);

        // New order [key 2, key 5 (new), key 0]: key 2 and key 0 reuse, key 5 is created, key 1
        // has no new home and is dropped.
        with_ctx(|ctx| {
            element.update(
                keyed_probes(&[(91, 2), (92, 5), (93, 0)], &m, &u),
                &mut render,
                ctx,
            );
        });

        assert_eq!(
            m.get(),
            4,
            "three initial mounts plus the one created child"
        );
        // Reused elements followed their keys; the created child carries its own mounted id; key 1
        // (mounted_id 20) is gone.
        assert_eq!(mounted_ids(&element), vec![30, 92, 10]);
        assert_eq!(child_ids(&element), vec![91, 92, 93]);
        // The render children match the element reorder, including the newly created child.
        assert_eq!(render_mounted_ids(&render), vec![30, 92, 10]);
    }

    /// A keyed widget whose element is itself a [`MultiChildElement`], so nesting two of them lets a
    /// reorder of the outer list recurse into a reorder of an inner list.
    struct Group {
        key: u32,
        children: Vec<Probe>,
    }

    impl Widget for Group {
        type Element = MultiChildElement<ProbeElement, MultiChildRenderList<ProbeRender>>;

        type Render = MultiChildRenderList<ProbeRender>;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let mut render = render_list();
            let element = MultiChildElement::new(self.children, &mut render, ctx);

            (element, render)
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.update(self.children, render, ctx);
        }

        fn key(&self) -> Option<&dyn AnyKeyable> {
            Some(&self.key)
        }
    }

    #[test]
    fn nested_reorder_does_not_reentrantly_borrow_the_keyed_map() {
        let (m, u) = (counter(), counter());
        let group = |key, items: &[(u32, u32)]| Group {
            key,
            children: keyed_probes(items, &m, &u),
        };

        let mut render = MultiChildRenderList::<MultiChildRenderList<ProbeRender>> {
            children: Vec::new(),
        };

        // Two keyed groups, each a keyed child list.
        let mut element: MultiChildElement<
            MultiChildElement<ProbeElement, MultiChildRenderList<ProbeRender>>,
            MultiChildRenderList<MultiChildRenderList<ProbeRender>>,
        > = with_ctx(|ctx| {
            MultiChildElement::new(
                vec![
                    group(0, &[(100, 0), (101, 1)]),
                    group(1, &[(200, 0), (201, 1)]),
                ],
                &mut render,
                ctx,
            )
        });
        assert_eq!(m.get(), 4, "four leaf probes mounted across the two groups");

        // Reorder the groups (key 1 first) AND reorder the probes inside each group. The outer
        // reorder recurses into each inner reorder while applying its plan.
        with_ctx(|ctx| {
            element.update(
                vec![
                    group(1, &[(210, 1), (211, 0)]),
                    group(0, &[(110, 1), (111, 0)]),
                ],
                &mut render,
                ctx,
            );
        });

        assert_eq!(m.get(), 4, "everything reused by key, nothing remounted");

        // Each element instance followed its key down both levels: group key 1 to outer index 0
        // with its probes reordered key-1-first, group key 0 to index 1.
        let nested: Vec<Vec<u32>> = element
            .children
            .iter()
            .map(|g| {
                g.node
                    .element
                    .children
                    .iter()
                    .map(|n| n.node.element.mounted_id)
                    .collect()
            })
            .collect();
        assert_eq!(nested, vec![vec![201, 200], vec![101, 100]]);
    }
}
