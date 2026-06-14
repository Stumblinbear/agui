use std::{any::TypeId, marker::PhantomData, ops::Range};

use rustc_hash::{FxBuildHasher, FxHashMap};

use crate::{
    context::{Dispatch, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, RoutingId, RoutingPath, node::ElementNode},
    key::AnyKeyable,
    render_object::{RenderObject, SingleChildRenderObject, node::RenderNode},
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

    fn dispatch(&mut self, render: &mut R, path: &RoutingPath, action: Dispatch) {
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

/// A flat, list of child elements with a routing id per child that stays with the child it
/// addresses. Reconciling reuses, drops, and creates children while keeping each survivor's id,
/// so a dispatch captured before a reorder still reaches the same element. Both the [`WidgetSequence`]
/// `Vec` and a widget that manages its own children (a paragraph laying out inline spans) hold one of
/// these.
///
/// [`WidgetSequence`]: crate::widget::WidgetSequence
pub struct MultiChildElement<C> {
    nodes: Vec<KeyedNode<C>>,
    next_id: RoutingId,
}

/// A child element paired with the type and key its widget reported at build, and the routing id that
/// addresses it for as long as it lives.
struct KeyedNode<C> {
    pub(crate) node: ElementNode<C>,
    type_id: TypeId,
    key: Option<Box<dyn AnyKeyable>>,
    id: RoutingId,
}

impl<C: Element> MultiChildElement<C> {
    /// Builds an element and render object per child, returning the storage and the parallel render
    /// nodes.
    pub fn build<CV>(children: Vec<CV>, ctx: &mut UpdateCtx) -> (Self, Vec<RenderNode<CV::Render>>)
    where
        CV: Widget<Element = C> + 'static,
    {
        let mut nodes = Vec::with_capacity(children.len());
        let mut render_children = Vec::with_capacity(children.len());

        let mut next_id = RoutingId::new(0);

        for child in children {
            let type_id = child.widget_type_id();
            let key = child.key().map(AnyKeyable::dyn_clone);
            let id = next_id.next();

            let (element, render_object) = ctx.with_routing_id(id, |ctx| child.create(ctx));

            nodes.push(KeyedNode {
                node: ElementNode::new(element),
                type_id,
                key,
                id,
            });
            render_children.push(RenderNode::new(render_object));
        }

        (Self { nodes, next_id }, render_children)
    }

    /// Reconciles the stored elements and `old_render` against `new`, returning the new render nodes in
    /// matching order. A child created here is grafted onto the mounted tree and mounted at once.
    pub fn reconcile<CV>(
        &mut self,
        new: Vec<CV>,
        old_render: Vec<RenderNode<CV::Render>>,
        ctx: &mut UpdateCtx,
    ) -> Vec<RenderNode<CV::Render>>
    where
        CV: Widget<Element = C> + 'static,
        CV::Render: RenderObject,
    {
        let old = std::mem::take(&mut self.nodes);

        let (nodes, render_children) =
            reconcile::<C, CV>(old, old_render, new, &mut self.next_id, ctx);

        self.nodes = nodes;
        render_children
    }

    /// The current index of the child addressed by `id`, if it is still present.
    fn child_index(&self, id: RoutingId) -> Option<usize> {
        // Ids are issued in order, so a child that has never moved sits at the index matching its id.
        let guess = id.get() as usize;

        if self.nodes.get(guess).is_some_and(|keyed| keyed.id == id) {
            return Some(guess);
        }

        self.nodes.iter().position(|keyed| keyed.id == id)
    }

    /// Routes `action` along `path` to the child the leading id names, threading the matching render
    /// node. A path addressing a child that has since been removed is dropped.
    pub fn dispatch(
        &mut self,
        renders: &mut [RenderNode<C::Render>],
        path: &RoutingPath,
        action: Dispatch,
    ) where
        C::Render: Sized,
    {
        let Some((head, rest)) = path.decode() else {
            unreachable!("multi-child element addresses one of its children");
        };

        // The addressed child may have been removed since the path was captured; drop the dispatch.
        let Some(index) = self.child_index(head) else {
            return;
        };

        self.nodes[index]
            .node
            .element
            .dispatch(&mut renders[index].object, rest, action);
    }
}

/// Reconciles `new` against the old child elements and their render objects, returning the new element
/// list and render-object list in matching order. Old children left unmatched are dropped.
fn reconcile<C, CV>(
    old: Vec<KeyedNode<C>>,
    old_render: Vec<RenderNode<CV::Render>>,
    new: Vec<CV>,
    next_id: &mut RoutingId,
    ctx: &mut UpdateCtx,
) -> (Vec<KeyedNode<C>>, Vec<RenderNode<CV::Render>>)
where
    C: Element,
    CV: Widget<Element = C> + 'static,
    CV::Render: RenderObject,
{
    let new_len = new.len();
    let old_len = old.len();

    let mut out_nodes = Vec::with_capacity(new_len);
    let mut out_render = Vec::with_capacity(new_len);

    // No children now: unmount every old child before it drops.
    if new_len == 0 {
        for mut render in old_render {
            ctx.unmount(&mut render.object);
        }

        return (out_nodes, out_render);
    }

    // No children before: materialize all of them.
    if old_len == 0 {
        for child in new {
            create(&mut out_nodes, &mut out_render, next_id, child, ctx);
        }

        return (out_nodes, out_render);
    }

    // The common stable-list update: same length, every child reusable where it sits. Reconcile in
    // place and hand back the old vectors, sparing the slot and output allocations the keyed scan needs.
    if old_len == new_len
        && old
            .iter()
            .zip(&new)
            .all(|(keyed, child)| node_can_update(keyed, child))
    {
        return reuse_all_in_place(old, old_render, new, ctx);
    }

    let mut old_slots: Vec<Slot<C, CV::Render>> =
        old.into_iter().zip(old_render).map(Some).collect();

    // Leading children that line up positionally.
    let mut prefix = 0;
    while prefix < old_len && prefix < new_len && can_update(&old_slots, &new, prefix, prefix) {
        prefix += 1;
    }

    // Trailing children that line up from the bottom.
    let mut new_bottom = new_len;
    let mut old_bottom = old_len;
    while prefix < old_bottom
        && prefix < new_bottom
        && can_update(&old_slots, &new, old_bottom - 1, new_bottom - 1)
    {
        old_bottom -= 1;
        new_bottom -= 1;
    }

    // Plan the keyed middle while `new` can still be borrowed; the loops below then move each child out
    // in ascending order, which is the only order they are consumed.
    let plan = (prefix < old_bottom && prefix < new_bottom)
        .then(|| match_keyed_middle(&old_slots, &new, prefix..old_bottom, prefix..new_bottom));

    let mut new = new.into_iter();

    for index in 0..prefix {
        let child = new.next().expect("prefix child");

        reuse(
            &mut out_nodes,
            &mut out_render,
            &mut old_slots,
            index,
            child,
            ctx,
        );
    }

    // The middle is matched by key: a matched new child reuses its old one, an unmatched new child is
    // created, and an old child no entry claims is left in `old_slots` to unmount below.
    if let Some(plan) = plan {
        for matched in plan {
            let child = new.next().expect("middle child");

            if let Some(old_index) = matched {
                reuse(
                    &mut out_nodes,
                    &mut out_render,
                    &mut old_slots,
                    old_index,
                    child,
                    ctx,
                );
            } else {
                create(&mut out_nodes, &mut out_render, next_id, child, ctx);
            }
        }
    } else {
        // No old children span the middle, so every new child there is created.
        for _ in 0..(new_bottom - prefix) {
            let child = new.next().expect("middle child");
            create(&mut out_nodes, &mut out_render, next_id, child, ctx);
        }
    }

    for offset in 0..(new_len - new_bottom) {
        let child = new.next().expect("suffix child");

        reuse(
            &mut out_nodes,
            &mut out_render,
            &mut old_slots,
            old_bottom + offset,
            child,
            ctx,
        );
    }

    // Any old child no new child claimed is leaving the tree; unmount it before it drops.
    for (_, mut render) in old_slots.into_iter().flatten() {
        ctx.unmount(&mut render.object);
    }

    (out_nodes, out_render)
}

type Slot<C, R> = Option<(KeyedNode<C>, RenderNode<R>)>;

type OldSlots<C, R> = [Slot<C, R>];

/// Whether the old child at `old_index` can be reconciled in place by the new child at `new_index`.
fn can_update<C, CV>(
    old_slots: &OldSlots<C, CV::Render>,
    new: &[CV],
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

    node_can_update(keyed, &new[new_index])
}

/// Whether the old `keyed` child can be reconciled in place by the new `child` widget: same widget type
/// and same key.
fn node_can_update<C, CV>(keyed: &KeyedNode<C>, child: &CV) -> bool
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    keyed.type_id == child.widget_type_id() && key_eq(child.key(), keyed.key.as_deref())
}

/// Reuses the old child at `old_index` for the next new slot, reconciling it in place under its existing
/// routing id and appending it to the result lists.
fn reuse<C, CV>(
    out_nodes: &mut Vec<KeyedNode<C>>,
    out_render: &mut Vec<RenderNode<CV::Render>>,
    old_slots: &mut OldSlots<C, CV::Render>,
    old_index: usize,
    child: CV,
    ctx: &mut UpdateCtx,
) where
    C: Element,
    CV: Widget<Element = C>,
{
    let (mut keyed, mut render) = old_slots[old_index].take().expect("reused a slot twice");

    // The stored key already equals the new child's: reuse is gated on `node_can_update`, which compares
    // them. So there is nothing to re-store here.
    ctx.with_routing_id(keyed.id, |ctx| {
        child.update(&mut keyed.node.element, &mut render.object, ctx);
    });

    out_nodes.push(keyed);
    out_render.push(render);
}

/// Reconciles every new child against the old child at the same index, reusing the old element and
/// render-object vectors as the result. The caller guarantees equal length and that each position passes
/// [`node_can_update`].
fn reuse_all_in_place<C, CV>(
    mut out_nodes: Vec<KeyedNode<C>>,
    mut out_render: Vec<RenderNode<CV::Render>>,
    new: Vec<CV>,
    ctx: &mut UpdateCtx,
) -> (Vec<KeyedNode<C>>, Vec<RenderNode<CV::Render>>)
where
    C: Element,
    CV: Widget<Element = C>,
{
    for (index, child) in new.into_iter().enumerate() {
        let id = out_nodes[index].id;

        ctx.with_routing_id(id, |ctx| {
            child.update(
                &mut out_nodes[index].node.element,
                &mut out_render[index].object,
                ctx,
            );
        });
    }

    (out_nodes, out_render)
}

/// Builds a fresh element and render object for the new `child` under a newly allocated routing id,
/// appending both to the result lists.
fn create<C, CV>(
    out_nodes: &mut Vec<KeyedNode<C>>,
    out_render: &mut Vec<RenderNode<CV::Render>>,
    next_id: &mut RoutingId,
    child: CV,
    ctx: &mut UpdateCtx,
) where
    C: Element,
    CV: Widget<Element = C> + 'static,
    CV::Render: RenderObject,
{
    let type_id = child.widget_type_id();
    let key = child.key().map(AnyKeyable::dyn_clone);
    let id = next_id.next();

    let (element, mut render_object) = ctx.with_routing_id(id, |ctx| child.create(ctx));

    // A subtree grafted onto the mounted tree is mounted here; its own mount cascades to its children.
    ctx.mount(&mut render_object);

    out_nodes.push(KeyedNode {
        node: ElementNode::new(element),
        type_id,
        key,
        id,
    });
    out_render.push(RenderNode::new(render_object));
}

/// For each keyed new child in `new_range`, the old index in `old_range` it reuses by key, or `None` to
/// create one.
fn match_keyed_middle<C, CV>(
    old_slots: &OldSlots<C, CV::Render>,
    new: &[CV],
    old_range: Range<usize>,
    new_range: Range<usize>,
) -> Vec<Option<usize>>
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    #[allow(clippy::mutable_key_type)]
    let mut old_keyed: FxHashMap<&dyn AnyKeyable, usize> =
        FxHashMap::with_capacity_and_hasher(old_range.len(), FxBuildHasher);

    for index in old_range {
        if let Some((keyed, _)) = old_slots[index].as_ref()
            && let Some(key) = keyed.key.as_deref()
        {
            old_keyed.insert(key, index);
        }
    }

    new_range
        .map(|index| new[index].key().and_then(|key| old_keyed.remove(&key)))
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
        context::{Dispatch, MessageCtx, MountCtx, UpdateCtx},
        element::{Element, RoutingId, RoutingPath},
        key::AnyKeyable,
        render_object::{RenderObject, node::RenderNode},
        test_harness::TestCtx,
        widget::Widget,
    };

    use super::{MultiChildElement, SingleChildElement};

    struct Probe {
        id: u32,
        key: Option<u32>,
        mounts: Rc<Cell<usize>>,
        updates: Rc<Cell<usize>>,
        unmounts: Rc<Cell<usize>>,
    }

    struct ProbeElement {
        id: u32,
        mounted_id: u32,
    }

    /// The render object of a [`Probe`], stamped with its mount-time id so a reorder can be checked
    /// against the render tree as well as the element tree.
    struct ProbeRender {
        mounted_id: u32,
        unmounts: Rc<Cell<usize>>,
    }

    impl Element for ProbeElement {
        type Render = ProbeRender;

        fn dispatch(&mut self, _: &mut ProbeRender, path: &RoutingPath, action: Dispatch) {
            assert!(path.is_empty(), "probe is a leaf");

            if let Dispatch::Message(ctx) = action {
                ctx.consume::<Rc<Cell<Option<u32>>>>()
                    .set(Some(self.mounted_id));
            }
        }
    }

    impl RenderObject for ProbeRender {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {
            self.unmounts.set(self.unmounts.get() + 1);
        }

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
                    unmounts: self.unmounts,
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
                unmounts: Rc::new(Cell::new(0)),
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
                unmounts: Rc::new(Cell::new(0)),
            })
            .collect()
    }

    /// Build probes that tally their render objects' unmounts into `unmounts`, so a removal can be
    /// observed on the render side.
    fn tracked_probes(
        ids: &[u32],
        mounts: &Rc<Cell<usize>>,
        updates: &Rc<Cell<usize>>,
        unmounts: &Rc<Cell<usize>>,
    ) -> Vec<Probe> {
        ids.iter()
            .map(|&id| Probe {
                id,
                key: None,
                mounts: Rc::clone(mounts),
                updates: Rc::clone(updates),
                unmounts: Rc::clone(unmounts),
            })
            .collect()
    }

    /// A keyed child list and its parallel render nodes, the pair a multi-child element holds.
    struct Children {
        elements: MultiChildElement<ProbeElement>,
        render: Vec<RenderNode<ProbeRender>>,
    }

    impl Children {
        fn new(children: Vec<Probe>, ctx: &mut UpdateCtx) -> Self {
            let (elements, render) = MultiChildElement::build(children, ctx);
            Self { elements, render }
        }

        fn update(&mut self, children: Vec<Probe>, ctx: &mut UpdateCtx) {
            let old_render = std::mem::take(&mut self.render);
            self.render = self.elements.reconcile(children, old_render, ctx);
        }
    }

    fn child_ids(children: &Children) -> Vec<u32> {
        children
            .elements
            .nodes
            .iter()
            .map(|n| n.node.element.id)
            .collect()
    }

    /// The mount-stamped identities in order, showing which element instance sits at each position.
    fn mounted_ids(children: &Children) -> Vec<u32> {
        children
            .elements
            .nodes
            .iter()
            .map(|n| n.node.element.mounted_id)
            .collect()
    }

    /// The mount-stamped identities of the render children, to check render/element lockstep.
    fn render_mounted_ids(children: &Children) -> Vec<u32> {
        children
            .render
            .iter()
            .map(|n| n.object.mounted_id)
            .collect()
    }

    #[test]
    fn mount_materializes_each_child() {
        let (m, u) = (counter(), counter());
        let children = TestCtx::new().run(|ctx| Children::new(probes(&[10, 20, 30], &m, &u), ctx));

        assert_eq!(m.get(), 3);
        assert_eq!(u.get(), 0);
        assert_eq!(child_ids(&children), vec![10, 20, 30]);
        assert_eq!(render_mounted_ids(&children), vec![10, 20, 30]);
    }

    #[test]
    fn update_same_length_reuses_each_child_in_place() {
        let (m, u) = (counter(), counter());
        let mut children = TestCtx::new().run(|ctx| Children::new(probes(&[1, 2, 3], &m, &u), ctx));

        TestCtx::new().run(|ctx| children.update(probes(&[4, 5, 6], &m, &u), ctx));

        assert_eq!(m.get(), 3, "no children remounted");
        assert_eq!(u.get(), 3, "each child reconciled in place");
        assert_eq!(child_ids(&children), vec![4, 5, 6]);
        assert_eq!(mounted_ids(&children), vec![1, 2, 3]);
        assert_eq!(render_mounted_ids(&children), vec![1, 2, 3]);
    }

    #[test]
    fn appending_only_mounts_the_new_tail() {
        let (m, u) = (counter(), counter());
        let mut children = TestCtx::new().run(|ctx| Children::new(probes(&[1, 2], &m, &u), ctx));

        TestCtx::new().run(|ctx| children.update(probes(&[1, 2, 3], &m, &u), ctx));

        assert_eq!(m.get(), 3, "two initial mounts plus one appended");
        assert_eq!(u.get(), 2, "the two retained children updated");
        assert_eq!(child_ids(&children), vec![1, 2, 3]);
        assert_eq!(render_mounted_ids(&children), vec![1, 2, 3]);
    }

    #[test]
    fn truncating_drops_the_extra_tail() {
        let (m, u) = (counter(), counter());
        let mut children = TestCtx::new().run(|ctx| Children::new(probes(&[1, 2, 3], &m, &u), ctx));

        TestCtx::new().run(|ctx| children.update(probes(&[1, 2], &m, &u), ctx));

        assert_eq!(m.get(), 3, "nothing new mounted");
        assert_eq!(u.get(), 2, "the two survivors updated");
        assert_eq!(child_ids(&children), vec![1, 2]);
        assert_eq!(render_mounted_ids(&children), vec![1, 2]);
    }

    #[test]
    fn updating_to_empty_clears_children() {
        let (m, u) = (counter(), counter());
        let mut children = TestCtx::new().run(|ctx| Children::new(probes(&[1, 2], &m, &u), ctx));

        TestCtx::new().run(|ctx| children.update(probes(&[], &m, &u), ctx));

        assert!(child_ids(&children).is_empty());
        assert!(children.render.is_empty());
    }

    #[test]
    fn truncating_unmounts_the_dropped_tail() {
        let (m, u, un) = (counter(), counter(), counter());
        let mut children =
            TestCtx::new().run(|ctx| Children::new(tracked_probes(&[1, 2, 3], &m, &u, &un), ctx));

        TestCtx::new().run(|ctx| children.update(tracked_probes(&[1, 2], &m, &u, &un), ctx));

        assert_eq!(
            un.get(),
            1,
            "the dropped child is unmounted, not merely released"
        );
        assert_eq!(child_ids(&children), vec![1, 2]);
    }

    #[test]
    fn updating_to_empty_unmounts_every_child() {
        let (m, u, un) = (counter(), counter(), counter());
        let mut children =
            TestCtx::new().run(|ctx| Children::new(tracked_probes(&[1, 2, 3], &m, &u, &un), ctx));

        TestCtx::new().run(|ctx| children.update(tracked_probes(&[], &m, &u, &un), ctx));

        assert_eq!(
            un.get(),
            3,
            "every child is unmounted before the list is cleared"
        );
        assert!(children.render.is_empty());
    }

    #[test]
    fn updating_from_empty_materializes_children() {
        let (m, u) = (counter(), counter());
        let mut children = TestCtx::new().run(|ctx| Children::new(probes(&[], &m, &u), ctx));
        assert!(child_ids(&children).is_empty());

        TestCtx::new().run(|ctx| children.update(probes(&[7, 8], &m, &u), ctx));

        assert_eq!(m.get(), 2, "fresh children are mounted");
        assert_eq!(u.get(), 0, "none are updated");
        assert_eq!(child_ids(&children), vec![7, 8]);
        assert_eq!(render_mounted_ids(&children), vec![7, 8]);
    }

    #[test]
    fn single_child_reuses_its_element_across_update() {
        let (m, u) = (counter(), counter());
        let (mut element, mut render): (
            SingleChildElement<ProbeElement, ProbeRender>,
            ProbeRender,
        ) = TestCtx::new().run(|ctx| {
            SingleChildElement::new(
                Probe {
                    id: 1,
                    key: None,
                    mounts: Rc::clone(&m),
                    updates: Rc::clone(&u),
                    unmounts: Rc::new(Cell::new(0)),
                },
                ctx,
            )
        });
        assert_eq!(m.get(), 1);
        assert_eq!(element.child.element.id, 1);

        TestCtx::new().run(|ctx| {
            element.update(
                Probe {
                    id: 9,
                    key: None,
                    mounts: Rc::clone(&m),
                    updates: Rc::clone(&u),
                    unmounts: Rc::new(Cell::new(0)),
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

        // Two same-type keyed children: id 10 keyed 0, id 20 keyed 1.
        let mut children =
            TestCtx::new().run(|ctx| Children::new(keyed_probes(&[(10, 0), (20, 1)], &m, &u), ctx));
        assert_eq!(mounted_ids(&children), vec![10, 20]);
        assert_eq!(render_mounted_ids(&children), vec![10, 20]);

        // Reorder to [key 1, key 0] with fresh config ids, so we can see which element moved.
        TestCtx::new().run(|ctx| children.update(keyed_probes(&[(98, 1), (99, 0)], &m, &u), ctx));

        assert_eq!(m.get(), 2, "both elements reused, neither remounted");
        // State moved with the key: key 1 (mounted_id 20) to position 0, key 0 (mounted_id 10) to 1.
        assert_eq!(mounted_ids(&children), vec![20, 10]);
        // Config (new widget ids) lands in the new order.
        assert_eq!(child_ids(&children), vec![98, 99]);
        // The render children reordered in lockstep with the elements.
        assert_eq!(render_mounted_ids(&children), vec![20, 10]);
    }

    #[test]
    fn keyed_reorder_reuses_drops_and_creates_together() {
        let (m, u) = (counter(), counter());

        // Keys 0,1,2 mounted as ids 10,20,30.
        let mut children = TestCtx::new()
            .run(|ctx| Children::new(keyed_probes(&[(10, 0), (20, 1), (30, 2)], &m, &u), ctx));
        assert_eq!(mounted_ids(&children), vec![10, 20, 30]);

        // New order [key 2, key 5 (new), key 0]: key 2 and key 0 reuse, key 5 is created, key 1 has no
        // new home and is dropped.
        TestCtx::new()
            .run(|ctx| children.update(keyed_probes(&[(91, 2), (92, 5), (93, 0)], &m, &u), ctx));

        assert_eq!(
            m.get(),
            4,
            "three initial mounts plus the one created child"
        );
        // Reused elements followed their keys; the created child carries its own mounted id; key 1
        // (mounted_id 20) is gone.
        assert_eq!(mounted_ids(&children), vec![30, 92, 10]);
        assert_eq!(child_ids(&children), vec![91, 92, 93]);
        // The render children match the element reorder, including the newly created child.
        assert_eq!(render_mounted_ids(&children), vec![30, 92, 10]);
    }

    /// Delivers a message addressed by `id` into `children`, returning the `mounted_id` of the probe
    /// that received it, if any did.
    fn deliver(children: &mut Children, id: u32) -> Option<u32> {
        let received = Rc::new(Cell::new(None));
        let mut ctx = MessageCtx::new(Box::new(Rc::clone(&received)));

        let path = RoutingId::encode_path([RoutingId::new(id)]);
        children.elements.dispatch(
            &mut children.render,
            RoutingPath::new(&path),
            Dispatch::Message(&mut ctx),
        );

        received.get()
    }

    #[test]
    fn dispatch_after_keyed_reorder_reaches_the_moved_element() {
        let (m, u) = (counter(), counter());

        // Ids are allocated in mount order, so key 0's element holds id 0 and key 1's holds id 1.
        let mut children =
            TestCtx::new().run(|ctx| Children::new(keyed_probes(&[(10, 0), (20, 1)], &m, &u), ctx));

        TestCtx::new().run(|ctx| children.update(keyed_probes(&[(98, 1), (99, 0)], &m, &u), ctx));

        // Each id still reaches the element it was issued to, not whoever sits at that index now.
        assert_eq!(deliver(&mut children, 1), Some(20));
        assert_eq!(deliver(&mut children, 0), Some(10));
    }

    #[test]
    fn dispatch_to_a_removed_child_is_dropped() {
        let (m, u) = (counter(), counter());

        let mut children =
            TestCtx::new().run(|ctx| Children::new(keyed_probes(&[(10, 0), (20, 1)], &m, &u), ctx));

        TestCtx::new().run(|ctx| children.update(keyed_probes(&[(99, 1)], &m, &u), ctx));

        assert_eq!(
            deliver(&mut children, 0),
            None,
            "the dropped child's id no longer delivers"
        );
        assert_eq!(deliver(&mut children, 1), Some(20));
    }

    #[test]
    fn created_child_gets_a_fresh_id() {
        let (m, u) = (counter(), counter());

        let mut children =
            TestCtx::new().run(|ctx| Children::new(keyed_probes(&[(10, 0), (20, 1)], &m, &u), ctx));

        // Key 5 is new: it is created under the next id, 2, never one freed by the dropped children.
        TestCtx::new().run(|ctx| children.update(keyed_probes(&[(91, 5)], &m, &u), ctx));

        assert_eq!(deliver(&mut children, 0), None);
        assert_eq!(deliver(&mut children, 1), None);
        assert_eq!(deliver(&mut children, 2), Some(91));
    }
}
