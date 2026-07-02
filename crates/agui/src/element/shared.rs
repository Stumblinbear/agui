//! The shared elements ordinary widgets reuse instead of writing their own. [`SingleChildElement`] holds
//! one inline child; [`MultiChildElement`] holds a flat, keyed run of dynamic children. Both keep their
//! children in [`agui_core::tree`] slots and reconcile them in place, so a widget supplies only its recipe.

use std::any::TypeId;
use std::ops::Range;

use rustc_hash::{FxBuildHasher, FxHashMap};

use agui_core::tree::{BoxedSlot, Slot};

use crate::{
    context::{CreateCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode, DiagnosticsNodeBuilder},
    element::Element,
    key::AnyKeyable,
    pipeline::render_pipeline::LayoutScope,
    render_object::{
        SingleChildRenderObject,
        box_layout::RenderBox,
        node::{RenderNode, RenderObjectCell, RenderObjectPtr},
    },
    widget::Widget,
};

/// The [`Element`] of a widget with a single child, such as `Padding` or `SizedBox`, threading the widget's
/// render `R` to the child. It holds the child inline and forwards its own lifecycle straight to it.
pub struct SingleChildElement<C, R> {
    child: Slot<C>,
    render: RenderObjectCell<R>,
}

impl<C: Element, R> SingleChildElement<C, R> {
    /// Builds the element from `child` and the `render` object the widget produced. The render object's child
    /// edge is left unwired until mount, when the child is pinned.
    pub fn new<Child>(ctx: &mut CreateCtx, child: Child, render: R) -> Self
    where
        Child: Widget<Element = C>,
    {
        let child_element = child.create(ctx);

        SingleChildElement {
            child: Slot::new(child_element),
            render: RenderObjectCell::new(render),
        }
    }

    /// Reconciles the child element against `child`.
    pub fn update<Child>(&mut self, ctx: &mut UpdateCtx<'_>, child: Child)
    where
        Child: Widget<Element = C>,
    {
        // SAFETY: `self.child` is our own slot.
        unsafe {
            ctx.with_child(&mut self.child, |element, ctx| {
                child.update(ctx, element);
            });
        }
    }

    /// This element's render object, by exclusive reference, for the widget's own writes during reconcile.
    pub fn render_object_mut(&mut self) -> &mut R {
        self.render.get_mut()
    }
}

// SAFETY: manages its single child only through the cursor child operations, and resolves its render object
// from its own `RenderObjectCell`.
unsafe impl<C, R> Element for SingleChildElement<C, R>
where
    C: Element,
    R: SingleChildRenderObject<Child = C::Render>,
{
    type Render = R;

    fn render_object_ptr(&self) -> RenderObjectPtr<R> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        let child = unsafe { ctx.mount(&mut self.child) };
        let render = self.render.get_mut();
        render.adopt_child(child);
        render.attach(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().detach(ctx);
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.get().describe(d))
            .finish()
    }
}

/// A reusable mechanism for a flat, keyed run of dynamic children, held by an [`Element`] that has more than
/// one child, such as `Column`'s element. It builds, mounts, reconciles, and unmounts the run; the owning
/// element delegates its lifecycle hooks to it. Reconciling reuses, drops, and creates children while each
/// survivor keeps its slot, so its [`NodeHandle`](agui_core::tree::NodeHandle) and the state behind it follow
/// it across a reorder.
pub struct MultiChildElement<C> {
    children: Vec<KeyedChild<C>>,
}

/// A child element paired with the type and key its widget reported at build, the pair that decides whether
/// a new widget reconciles it in place. Its address is the [`NodeHandle`](agui_core::tree::NodeHandle) the
/// slot carries, stable for as long as the child lives.
struct KeyedChild<C> {
    child: BoxedSlot<C>,
    type_id: TypeId,
    key: Option<Box<dyn AnyKeyable>>,
}

impl<C: Element<Render = dyn RenderBox>> MultiChildElement<C> {
    /// Builds an element per child, returning the parallel render edges (unwired until mount) for the owning
    /// render object to hold. Ready to be mounted.
    pub fn new<CV>(ctx: &mut CreateCtx, children: Vec<CV>) -> (Self, Vec<RenderNode<dyn RenderBox>>)
    where
        CV: Widget<Element = C> + 'static,
    {
        let mut elements = Vec::with_capacity(children.len());
        let mut renders = Vec::with_capacity(children.len());

        for child in children {
            let type_id = child.widget_type_id();
            let key = child.key().map(AnyKeyable::dyn_clone);

            let element = child.create(ctx);
            elements.push(KeyedChild {
                child: BoxedSlot::new(element),
                type_id,
                key,
            });
            renders.push(RenderNode::new(()));
        }

        (Self { children: elements }, renders)
    }

    /// Reconciles the stored children and their render edges against `new`, in lockstep. A child created here
    /// is grafted onto both, mounted, and its edge wired at once; one no new child claims is unmounted.
    pub fn update<CV>(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        new: Vec<CV>,
        render_children: &mut Vec<RenderNode<dyn RenderBox>>,
        layout_scope: LayoutScope,
    ) where
        CV: Widget<Element = C> + 'static,
    {
        let old = std::mem::take(&mut self.children)
            .into_iter()
            .zip(std::mem::take(render_children))
            .collect();
        let (elements, renders) = ctx.with_children(layout_scope, |ctx| {
            reconcile::<C, CV>(ctx, old, new, layout_scope)
        });
        self.children = elements;
        *render_children = renders;
    }

    /// Mounts every child element and wires its render edge into the matching `renders` slot, now that the
    /// child is pinned.
    pub fn mount(&mut self, ctx: &mut UpdateCtx<'_>, renders: &mut Vec<RenderNode<dyn RenderBox>>) {
        for (keyed, render) in self.children.iter_mut().zip(renders.iter_mut()) {
            // SAFETY: each child is a `BoxedSlot`, whose body ignores the cursor's base, so it registers
            // soundly under `ctx`; the run deregisters every child before it drops.
            let mounted = unsafe { ctx.mount(&mut keyed.child) };
            render.set(mounted);
        }
    }

    /// Unmounts every child element.
    pub fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        for keyed in &mut self.children {
            // SAFETY: as `mount`.
            unsafe { ctx.unmount(&mut keyed.child) };
        }
    }

    /// Threads each child's diagnostics under `node`, in order.
    pub fn describe_children<'a>(
        &self,
        node: DiagnosticsNodeBuilder<'a>,
    ) -> DiagnosticsNodeBuilder<'a> {
        self.children.iter().fold(node, |node, keyed| {
            node.child(|d| keyed.child.get().describe(d))
        })
    }
}

/// An element child and its render edge, reconciled together so a keyed reorder moves both in lockstep
/// without a separate index to keep aligned.
type Pair<C> = (KeyedChild<C>, RenderNode<dyn RenderBox>);

/// Reconciles `new` against the old element/render pairs, returning the new pairs in matching order, split
/// back into the element list and the render list. Old children no new child claims are unmounted from both
/// trees.
fn reconcile<C, CV>(
    ctx: &mut UpdateCtx<'_>,
    old: Vec<Pair<C>>,
    new: Vec<CV>,
    layout_scope: LayoutScope,
) -> (Vec<KeyedChild<C>>, Vec<RenderNode<dyn RenderBox>>)
where
    C: Element<Render = dyn RenderBox>,
    CV: Widget<Element = C> + 'static,
{
    let new_len = new.len();
    let old_len = old.len();

    let mut out: Vec<Pair<C>> = Vec::with_capacity(new_len);

    // No children now: unmount every old child.
    if new_len == 0 {
        ctx.mark_needs_layout(layout_scope);

        for mut pair in old {
            unmount_pair(ctx, &mut pair);
        }

        return split(out);
    }

    // No children before: materialize all of them.
    if old_len == 0 {
        ctx.mark_needs_layout(layout_scope);

        for child in new {
            out.push(create(ctx, child));
        }

        return split(out);
    }

    // The common stable-list update: same length, every child reusable where it sits. Reconcile in place
    // and hand back the old vector, sparing the slot and output allocations the keyed scan needs.
    if old_len == new_len
        && old
            .iter()
            .zip(&new)
            .all(|(pair, child)| node_can_update(&pair.0, child))
    {
        return split(reuse_all_in_place(ctx, old, new));
    }

    // The sibling list changed structurally. Re-lay the enclosing boundary (a grafted child is unlaid, and the
    // mark must come from here because, unlike semantics, layout has no per-child `attach`/`detach` hook to do
    // it). Then re-walk the new order into semantics; add/remove there are already marked via `attach`/`detach`,
    // so only the reorder needs this.
    ctx.mark_needs_layout(layout_scope);
    ctx.mark_needs_semantics_update();

    let mut old_slots: Vec<Option<Pair<C>>> = old.into_iter().map(Some).collect();

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

    // Plan the keyed middle while `new` can still be borrowed; the loops below then move each child out in
    // ascending order, which is the only order they are consumed.
    let plan = (prefix < old_bottom && prefix < new_bottom)
        .then(|| match_keyed_middle(&old_slots, &new, prefix..old_bottom, prefix..new_bottom));

    let mut new = new.into_iter();

    for index in 0..prefix {
        let child = new.next().expect("prefix child");
        reuse(ctx, &mut out, &mut old_slots, index, child);
    }

    // The middle is matched by key: a matched new child reuses its old one, an unmatched new child is
    // created, and an old child no entry claims is left in `old_slots` to unmount below.
    if let Some(plan) = plan {
        for matched in plan {
            let child = new.next().expect("middle child");

            if let Some(old_index) = matched {
                reuse(ctx, &mut out, &mut old_slots, old_index, child);
            } else {
                out.push(create(ctx, child));
            }
        }
    } else {
        // No old children span the middle, so every new child there is created.
        for _ in 0..(new_bottom - prefix) {
            let child = new.next().expect("middle child");
            out.push(create(ctx, child));
        }
    }

    for offset in 0..(new_len - new_bottom) {
        let child = new.next().expect("suffix child");
        reuse(ctx, &mut out, &mut old_slots, old_bottom + offset, child);
    }

    // Any old child no new child claimed is leaving the tree; unmount it from both trees.
    for mut pair in old_slots.into_iter().flatten() {
        unmount_pair(ctx, &mut pair);
    }

    split(out)
}

/// Splits the reconciled pairs back into the element list the [`MultiChildElement`] holds and the render
/// edge list the owning render object holds.
fn split<C>(pairs: Vec<Pair<C>>) -> (Vec<KeyedChild<C>>, Vec<RenderNode<dyn RenderBox>>) {
    pairs.into_iter().unzip()
}

/// Unmounts a leaving child's element. Its render edge is dropped with the pair; the edge owns nothing, so
/// there is nothing else to release.
fn unmount_pair<C: Element>(ctx: &mut UpdateCtx<'_>, pair: &mut Pair<C>) {
    // SAFETY: `pair.0.child` is this element's own slot.
    unsafe { ctx.unmount(&mut pair.0.child) };
}

type OldSlots<C> = [Option<Pair<C>>];

/// Whether the old child at `old_index` can be reconciled in place by the new child at `new_index`.
fn can_update<C, CV>(
    old_slots: &OldSlots<C>,
    new: &[CV],
    old_index: usize,
    new_index: usize,
) -> bool
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    let pair = old_slots[old_index]
        .as_ref()
        .expect("compared a taken slot");
    node_can_update(&pair.0, &new[new_index])
}

/// Whether the old `keyed` child can be reconciled in place by the new `child` widget: same widget type and
/// same key.
fn node_can_update<C, CV>(keyed: &KeyedChild<C>, child: &CV) -> bool
where
    C: Element,
    CV: Widget<Element = C> + 'static,
{
    keyed.type_id == child.widget_type_id() && key_eq(child.key(), keyed.key.as_deref())
}

/// Reuses the old child at `old_index` for the next new slot, reconciling its element and render node in
/// place and appending the pair to the result.
fn reuse<C, CV>(
    ctx: &mut UpdateCtx<'_>,
    out: &mut Vec<Pair<C>>,
    old_slots: &mut OldSlots<C>,
    old_index: usize,
    child: CV,
) where
    C: Element,
    CV: Widget<Element = C>,
{
    let mut pair = old_slots[old_index].take().expect("reused a slot twice");

    // Reuse is gated on `node_can_update`, which already compared the keys, so the stored key still matches.
    // The child keeps its slot and its render object, so its edge stays valid; only the element reconciles.
    // SAFETY: `pair.0.child` is this element's own slot.
    unsafe {
        ctx.with_child(&mut pair.0.child, |element, ctx| {
            child.update(ctx, element);
        });
    }

    out.push(pair);
}

/// Reconciles every new child against the old child at the same index, reusing the old vector as the
/// result. The caller guarantees equal length and that each position passes [`node_can_update`].
fn reuse_all_in_place<C, CV>(
    ctx: &mut UpdateCtx<'_>,
    mut out: Vec<Pair<C>>,
    new: Vec<CV>,
) -> Vec<Pair<C>>
where
    C: Element,
    CV: Widget<Element = C>,
{
    for (index, child) in new.into_iter().enumerate() {
        let pair = &mut out[index];
        // SAFETY: `pair.0.child` is this element's own slot.
        unsafe {
            ctx.with_child(&mut pair.0.child, |element, ctx| {
                child.update(ctx, element);
            });
        }
    }

    out
}

/// Builds a fresh element and render node for the new `child`, mounts the element, and returns the pair with
/// the child's identity. The render node is held by the owning render object; it carries no registry entry
/// unless it is later mounted as a boundary.
fn create<C, CV>(ctx: &mut UpdateCtx<'_>, child: CV) -> Pair<C>
where
    C: Element<Render = dyn RenderBox>,
    CV: Widget<Element = C> + 'static,
{
    let type_id = child.widget_type_id();
    let key = child.key().map(AnyKeyable::dyn_clone);

    let element = ctx.inflate(|ctx| child.create(ctx));
    let mut child = BoxedSlot::new(element);
    // SAFETY: `child` is this element's own freshly built slot.
    let mounted = unsafe { ctx.mount(&mut child) };

    let mut render: RenderNode<dyn RenderBox> = RenderNode::new(());
    render.set(mounted);

    (
        KeyedChild {
            child,
            type_id,
            key,
        },
        render,
    )
}

/// For each keyed new child in `new_range`, the old index in `old_range` it reuses by key, or `None` to
/// create one.
fn match_keyed_middle<C, CV>(
    old_slots: &OldSlots<C>,
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
        if let Some(pair) = old_slots[index].as_ref()
            && let Some(key) = pair.0.key.as_deref()
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
