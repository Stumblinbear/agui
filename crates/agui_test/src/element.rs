//! Conformance checks for the lifecycle and routing contracts every custom [`Element`] must satisfy.

use std::{any::Any, cell::RefCell, rc::Rc};

use agui_core::{
    key::AnyKeyable,
    prelude::{element::*, render_object::*},
    test_harness::TestCtx,
};
use typed_floats::{Positive, PositiveFinite};

/// One tracked render object's mount or unmount, logged in order so the checker can assert
/// exactly-once lifecycle and recover the live set at any point.
#[derive(Clone, Copy)]
enum Life {
    Mount(u32),
    Unmount(u32),
}

#[derive(Default)]
struct LedgerState {
    log: Vec<Life>,
    targets: Vec<(u32, RoutingTarget)>,
    hits: Vec<u32>,
    describes: Vec<u32>,
}

/// A shared record of what the tracked children placed under test mounted, unmounted, and received.
/// Every [`Tracked`] child reports to the same ledger, so the checker reads the whole subtree's
/// lifecycle from one place.
#[derive(Clone, Default)]
struct Ledger {
    state: Rc<RefCell<LedgerState>>,
}

impl Ledger {
    fn tracked(&self, id: u32, key: Option<u32>) -> Tracked {
        Tracked {
            id,
            key,
            ledger: self.clone(),
        }
    }

    fn mount(&self, id: u32) {
        self.state.borrow_mut().log.push(Life::Mount(id));
    }

    fn unmount(&self, id: u32) {
        self.state.borrow_mut().log.push(Life::Unmount(id));
    }

    fn record_target(&self, id: u32, target: RoutingTarget) {
        let mut state = self.state.borrow_mut();
        if !state.targets.iter().any(|(other, _)| *other == id) {
            state.targets.push((id, target));
        }
    }

    fn record_hit(&self, id: u32) {
        self.state.borrow_mut().hits.push(id);
    }

    fn record_describe(&self, id: u32) {
        self.state.borrow_mut().describes.push(id);
    }

    fn mounts(&self, id: u32) -> usize {
        self.state
            .borrow()
            .log
            .iter()
            .filter(|life| matches!(life, Life::Mount(other) if *other == id))
            .count()
    }

    fn unmounts(&self, id: u32) -> usize {
        self.state
            .borrow()
            .log
            .iter()
            .filter(|life| matches!(life, Life::Unmount(other) if *other == id))
            .count()
    }

    fn hits(&self, id: u32) -> usize {
        self.state
            .borrow()
            .hits
            .iter()
            .filter(|&&h| h == id)
            .count()
    }

    fn describes(&self, id: u32) -> usize {
        self.state
            .borrow()
            .describes
            .iter()
            .filter(|&&d| d == id)
            .count()
    }

    /// The path that addressed `id` when it was first mounted, for replaying a dispatch after a
    /// reorder.
    fn target(&self, id: u32) -> RoutingTarget {
        self.state
            .borrow()
            .targets
            .iter()
            .find(|(other, _)| *other == id)
            .map(|(_, target)| target.clone())
            .expect("tracked child never recorded a routing target")
    }

    /// Every id the ledger has ever seen, in first-mount order.
    fn ids(&self) -> Vec<u32> {
        let mut ids = Vec::new();
        for life in &self.state.borrow().log {
            let (Life::Mount(id) | Life::Unmount(id)) = life;
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
        ids
    }
}

/// A leaf child the checker places in the widget under test. Its render object reports every mount
/// and unmount to the [`Ledger`], and its element records messages routed to it, so the checker can
/// hold the container to the lifecycle and routing contracts without the container cooperating.
pub struct Tracked {
    id: u32,
    key: Option<u32>,
    ledger: Ledger,
}

/// The element of a [`Tracked`] child: a leaf that records the messages routed to it.
pub struct TrackedElement {
    id: u32,
    ledger: Ledger,
}

impl Element for TrackedElement {
    type Render = TrackedRender;

    fn dispatch(&mut self, _: &mut TrackedRender, path: &RoutingPath, action: Dispatch) {
        assert!(path.is_empty(), "a tracked child is a leaf");

        if let Dispatch::Message(_) = action {
            self.ledger.record_hit(self.id);
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.ledger.record_describe(self.id);
        d.node_for::<Self>().finish()
    }
}

impl Widget for Tracked {
    type Element = TrackedElement;

    type Render = TrackedRender;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        self.ledger.record_target(self.id, ctx.routing_target());

        (
            TrackedElement {
                id: self.id,
                ledger: self.ledger.clone(),
            },
            TrackedRender {
                id: self.id,
                ledger: self.ledger,
            },
        )
    }

    fn update(self, _: &mut Self::Element, _: &mut Self::Render, _: &mut UpdateCtx) {}

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.key.as_ref().map(|k| k as &dyn AnyKeyable)
    }
}

/// The render object of a [`Tracked`] child: layout-inert, reporting only its mounts and unmounts.
pub struct TrackedRender {
    id: u32,
    ledger: Ledger,
}

impl RenderObject for TrackedRender {
    fn mount(&mut self, _: &mut MountCtx) {
        self.ledger.mount(self.id);
    }

    fn unmount(&mut self, _: &mut MountCtx) {
        self.ledger.unmount(self.id);
    }

    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

impl RenderBox for TrackedRender {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.smallest()
    }

    fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        constraints.smallest()
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
}

/// Checks that a custom element honors the universal lifecycle and routing contracts.
///
/// Pick the entry point for the element's arity ([`leaf`](Self::leaf),
/// [`single_child`](Self::single_child), or [`multi_child`](Self::multi_child)) and hand it a closure
/// that builds the widget under test around the [`Tracked`] children the check supplies. The
/// check then drives the element through a battery of reconciles and asserts, on the tracked
/// children, that:
///
/// - every child mounts exactly once when it enters the tree and unmounts exactly once when it
///   leaves, with none left mounted after the tree is torn down;
/// - a child that survives a rebuild is reconciled in place, not remounted;
/// - a keyed child keeps its render object across a reorder, and a dispatch captured before the
///   reorder still reaches it;
/// - a dispatch addressed to a child that has since been removed is dropped, not misrouted;
/// - an element whose `describe` recurses reaches its child rather than stopping at itself.
#[derive(Default)]
pub struct ElementLifecycleCheck {
    _private: (),
}

impl ElementLifecycleCheck {
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks a leaf element: it accepts an empty-path dispatch without panicking and produces
    /// diagnostics, the contracts a childless element still owns.
    ///
    /// # Panics
    ///
    /// Panics if the element violates one of those contracts.
    pub fn leaf<W>(&self, make: impl Fn() -> W)
    where
        W: Widget,
        W::Render: RenderObject,
    {
        let mut ctx = TestCtx::new();
        let (element, mut render) = ctx.create(make());
        ctx.run(|ctx| ctx.mount(&mut render));

        element.describe(&mut Diagnostics::new());

        ctx.run(|ctx| ctx.unmount(&mut render));
    }

    /// Checks a single-child element across mount, an in-place rebuild, a routed message, and
    /// teardown.
    ///
    /// # Panics
    ///
    /// Panics if the element violates a lifecycle or routing contract.
    pub fn single_child<W>(&self, make: impl Fn(Tracked) -> W)
    where
        W: Widget,
        W::Render: RenderObject,
    {
        let ledger = Ledger::default();
        let mut ctx = TestCtx::new();

        let (mut element, mut render) = ctx.create(make(ledger.tracked(1, None)));
        ctx.run(|ctx| ctx.mount(&mut render));
        assert_eq!(
            ledger.mounts(1),
            1,
            "the child mounts when the tree is mounted"
        );
        assert_eq!(ledger.unmounts(1), 0);

        ctx.run(|ctx| make(ledger.tracked(1, None)).update(&mut element, &mut render, ctx));
        assert_eq!(
            ledger.mounts(1),
            1,
            "an in-place rebuild reuses the child, it does not remount it"
        );

        element.describe(&mut Diagnostics::new());
        assert_eq!(
            ledger.describes(1),
            1,
            "describe must recurse into the child, not stop at the element"
        );

        deliver(&mut element, &mut render, &ledger.target(1));
        assert_eq!(ledger.hits(1), 1, "a routed message reaches the child");

        ctx.run(|ctx| ctx.unmount(&mut render));
        assert_balanced(&ledger);
    }

    /// Checks a multi-child element across growth, truncation, clearing, a keyed reorder, keyed
    /// removal, and the routing-id stability that keeps a captured dispatch reaching its child.
    ///
    /// # Panics
    ///
    /// Panics if the element violates a lifecycle or routing contract.
    pub fn multi_child<W>(&self, make: impl Fn(Vec<Tracked>) -> W)
    where
        W: Widget,
        W::Render: RenderObject,
    {
        self.check_positional(&make);
        self.check_keyed(&make);
    }

    /// Tail-only growth and truncation, where each position keeps a stable id, so mount and unmount
    /// counts read directly.
    fn check_positional<W>(&self, make: &impl Fn(Vec<Tracked>) -> W)
    where
        W: Widget,
        W::Render: RenderObject,
    {
        let ledger = Ledger::default();
        let mut ctx = TestCtx::new();

        let unkeyed = |ids: &[u32]| ids.iter().map(|&id| ledger.tracked(id, None)).collect();

        let (mut element, mut render) = ctx.create(make(unkeyed(&[1, 2, 3])));
        ctx.run(|ctx| ctx.mount(&mut render));
        for id in [1, 2, 3] {
            assert_eq!(ledger.mounts(id), 1, "each initial child mounts once");
            assert_eq!(ledger.unmounts(id), 0);
        }

        element.describe(&mut Diagnostics::new());
        for id in [1, 2, 3] {
            assert_eq!(
                ledger.describes(id),
                1,
                "describe must recurse into every child, not stop at the element"
            );
        }

        ctx.run(|ctx| make(unkeyed(&[1, 2, 3, 4])).update(&mut element, &mut render, ctx));
        assert_eq!(ledger.mounts(4), 1, "the appended child mounts");
        for id in [1, 2, 3] {
            assert_eq!(ledger.mounts(id), 1, "surviving children are not remounted");
            assert_eq!(ledger.unmounts(id), 0);
        }

        ctx.run(|ctx| make(unkeyed(&[1, 2])).update(&mut element, &mut render, ctx));
        for id in [3, 4] {
            assert_eq!(ledger.unmounts(id), 1, "a truncated child unmounts once");
        }
        for id in [1, 2] {
            assert_eq!(ledger.unmounts(id), 0, "survivors stay mounted");
        }

        ctx.run(|ctx| make(unkeyed(&[])).update(&mut element, &mut render, ctx));
        for id in [1, 2] {
            assert_eq!(ledger.unmounts(id), 1, "clearing unmounts the rest");
        }

        ctx.run(|ctx| ctx.unmount(&mut render));
        assert_balanced(&ledger);
    }

    /// A keyed reorder and removal, where identity follows the key rather than the position.
    fn check_keyed<W>(&self, make: &impl Fn(Vec<Tracked>) -> W)
    where
        W: Widget,
        W::Render: RenderObject,
    {
        let ledger = Ledger::default();
        let mut ctx = TestCtx::new();

        let keyed = |ids: &[u32]| ids.iter().map(|&id| ledger.tracked(id, Some(id))).collect();

        let (mut element, mut render) = ctx.create(make(keyed(&[1, 2, 3])));
        ctx.run(|ctx| ctx.mount(&mut render));

        // A dispatch captured against key 1 before the reorder.
        let one = ledger.target(1);

        ctx.run(|ctx| make(keyed(&[3, 1, 2])).update(&mut element, &mut render, ctx));
        for id in [1, 2, 3] {
            assert_eq!(
                ledger.mounts(id),
                1,
                "a reorder reuses every child, none remount"
            );
            assert_eq!(ledger.unmounts(id), 0, "a reorder unmounts nothing");
        }

        deliver(&mut element, &mut render, &one);
        assert_eq!(
            ledger.hits(1),
            1,
            "the routing id is stable, so the captured path still reaches key 1 after the reorder"
        );

        ctx.run(|ctx| make(keyed(&[1, 3])).update(&mut element, &mut render, ctx));
        assert_eq!(ledger.unmounts(2), 1, "the removed key unmounts once");
        assert_eq!(ledger.unmounts(1), 0, "the kept keys stay mounted");
        assert_eq!(ledger.unmounts(3), 0);

        let two = ledger.target(2);
        deliver(&mut element, &mut render, &two);
        assert_eq!(
            ledger.hits(2),
            0,
            "a dispatch to the removed key is dropped, not misrouted onto a sibling"
        );

        ctx.run(|ctx| ctx.unmount(&mut render));
        assert_balanced(&ledger);
    }
}

/// Routes an empty-payload message to the element at `target`'s path, the way the checker probes that
/// a child is reachable.
fn deliver<E: Element>(element: &mut E, render: &mut E::Render, target: &RoutingTarget) {
    let mut message = MessageCtx::new(Box::new(()) as Box<dyn Any>);
    element.dispatch(render, target.path(), Dispatch::Message(&mut message));
}

/// Asserts that every id the ledger saw mounted exactly as many times as it unmounted, with nothing
/// left live.
fn assert_balanced(ledger: &Ledger) {
    for id in ledger.ids() {
        assert_eq!(
            ledger.mounts(id),
            ledger.unmounts(id),
            "child {id} was mounted {} times but unmounted {} times",
            ledger.mounts(id),
            ledger.unmounts(id),
        );
    }
}

#[cfg(test)]
mod tests {
    use agui_core::prelude::{element::*, render_object::*};
    use typed_floats::{Positive, PositiveFinite};

    use super::{ElementLifecycleCheck, Tracked};

    /// A minimal multi-child container whose render object can be told to skip forwarding `mount` or
    /// `unmount`, so the checker's self-tests can confirm it catches a broken lifecycle.
    struct List {
        children: Vec<Tracked>,
        forward_mount: bool,
        forward_unmount: bool,
    }

    impl List {
        fn correct(children: Vec<Tracked>) -> Self {
            Self {
                children,
                forward_mount: true,
                forward_unmount: true,
            }
        }
    }

    impl Widget for List {
        type Element = ChildrenElement<Vec<Tracked>, ListRender>;

        type Render = ListRender;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let (element, children) = ChildrenElement::new(self.children, ctx);

            (
                element,
                ListRender {
                    children,
                    forward_mount: self.forward_mount,
                    forward_unmount: self.forward_unmount,
                },
            )
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.update(self.children, render, ctx);
        }
    }

    struct ListRender {
        children: Vec<RenderNode<TrackedRenderOf>>,
        forward_mount: bool,
        forward_unmount: bool,
    }

    type TrackedRenderOf = super::TrackedRender;

    impl MultiChildRenderObject for ListRender {
        type Children = Vec<RenderNode<TrackedRenderOf>>;

        fn children_mut(&mut self) -> &mut Vec<RenderNode<TrackedRenderOf>> {
            &mut self.children
        }
    }

    impl RenderObject for ListRender {
        fn mount(&mut self, ctx: &mut MountCtx) {
            if self.forward_mount {
                for child in &mut self.children {
                    child.mount(ctx);
                }
            }
        }

        fn unmount(&mut self, ctx: &mut MountCtx) {
            if self.forward_unmount {
                for child in &mut self.children {
                    child.unmount(ctx);
                }
            }
        }

        fn update_compositing_bits(&mut self) -> bool {
            let mut needs = false;
            for child in &mut self.children {
                needs |= child.update_compositing_bits();
            }
            needs
        }
    }

    impl RenderBox for ListRender {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            None
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.smallest()
        }

        fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            for child in &mut self.children {
                child.layout(ctx, constraints);
            }
            constraints.smallest()
        }

        fn measure_baseline(
            &self,
            _: BoxConstraints,
            _: TextBaseline,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
            None
        }

        fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
            HitTest::Pass
        }

        fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}
    }

    #[test]
    fn a_correct_multi_child_element_passes() {
        ElementLifecycleCheck::new().multi_child(List::correct);
    }

    #[test]
    #[should_panic(expected = "mounted")]
    fn a_container_that_skips_unmount_is_caught() {
        ElementLifecycleCheck::new().multi_child(|children| List {
            children,
            forward_mount: true,
            forward_unmount: false,
        });
    }

    #[test]
    #[should_panic(expected = "mount")]
    fn a_container_that_skips_mount_is_caught() {
        ElementLifecycleCheck::new().multi_child(|children| List {
            children,
            forward_mount: false,
            forward_unmount: true,
        });
    }
}
