//! Conformance checks for the lifecycle and routing contracts every custom [`Element`] must satisfy.

use std::{cell::RefCell, rc::Rc};

use agui::{
    key::AnyKeyable,
    prelude::{element::*, render_object::*},
};

use crate::ElementTester;
use typed_floats::{Positive, PositiveFinite};

/// One tracked child's mount or unmount, logged in order so the checker can assert exactly-once
/// lifecycle and recover the live set at any point.
#[derive(Clone, Copy)]
enum Life {
    Mount(u32),
    Unmount(u32),
}

#[derive(Default)]
struct LedgerState {
    log: Vec<Life>,
    handles: Vec<(u32, NodeHandle)>,
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

    fn record_handle(&self, id: u32, handle: NodeHandle) {
        let mut state = self.state.borrow_mut();
        if !state.handles.iter().any(|(other, _)| *other == id) {
            state.handles.push((id, handle));
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

    /// The handle that addressed `id` when it was first mounted, for replaying a dispatch after a
    /// reorder.
    fn handle(&self, id: u32) -> NodeHandle {
        self.state
            .borrow()
            .handles
            .iter()
            .find(|(other, _)| *other == id)
            .map(|(_, handle)| *handle)
            .expect("tracked child never recorded a handle")
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

/// A leaf child the checker places in the widget under test. Its element reports every mount, unmount,
/// message, and describe to the [`Ledger`] and captures the handle that addresses it, so the checker can
/// hold the container to the lifecycle and routing contracts without the container cooperating.
pub struct Tracked {
    id: u32,
    key: Option<u32>,
    ledger: Ledger,
}

/// The element of a [`Tracked`] child: a leaf that records its own lifecycle and the messages routed to
/// it, and owns a layout-inert render object so it can sit in a box-laying container.
pub struct TrackedElement {
    id: u32,
    ledger: Ledger,
    render: RenderObjectCell<TrackedRender>,
}

// SAFETY: a leaf with no children to register; its render object is resolved from its own cell and lives
// as long as the element is mounted.
unsafe impl Element for TrackedElement {
    type Render = TrackedRender;

    fn render_object_ptr(&self) -> RenderObjectPtr<TrackedRender> {
        self.render.render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.ledger.mount(self.id);
        self.ledger.record_handle(self.id, ctx.handle());
        self.render.get_mut().attach(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.render.get_mut().detach(ctx);
        self.ledger.unmount(self.id);
    }

    fn message(&mut self, _ctx: &mut MessageCtx<'_>) {
        self.ledger.record_hit(self.id);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.ledger.record_describe(self.id);
        d.node_for::<Self>().finish()
    }
}

impl Widget for Tracked {
    type Element = TrackedElement;

    type Render = TrackedRender;

    fn create(self, _ctx: &mut CreateCtx) -> TrackedElement {
        TrackedElement {
            id: self.id,
            ledger: self.ledger,
            render: RenderObjectCell::new(TrackedRender),
        }
    }

    fn update(self, _ctx: &mut UpdateCtx<'_>, _element: &mut TrackedElement) {}

    fn key(&self) -> Option<&dyn AnyKeyable> {
        self.key.as_ref().map(|k| k as &dyn AnyKeyable)
    }
}

/// The render object of a [`Tracked`] child: layout-inert, contributing nothing to layout, paint, or
/// semantics, so the checker exercises only the element lifecycle around it.
pub struct TrackedRender;

impl RenderObject for TrackedRender {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
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

    fn update_compositing_bits(&mut self) -> bool {
        false
    }

    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {}

    fn build_semantics(&mut self, _: &mut SemanticsTreeBuilder<'_>) {}
}

/// Checks that a custom element honors the universal lifecycle and routing contracts.
///
/// Pick the entry point for the element's arity ([`leaf`](Self::leaf),
/// [`single_child`](Self::single_child), or [`multi_child`](Self::multi_child)) and hand it a closure
/// that builds the widget under test around the [`Tracked`] children the check supplies. The check then
/// drives the element through a battery of reconciles and asserts, on the tracked children, that:
///
/// - every child mounts exactly once when it enters the tree and unmounts exactly once when it leaves,
///   with none left mounted after the children are cleared;
/// - a child that survives a rebuild is reconciled in place, not remounted;
/// - a keyed child keeps its address across a reorder, and a dispatch captured before the reorder still
///   reaches it;
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

    /// Checks a leaf element: it mounts, produces diagnostics, and accepts a routed message without
    /// panicking, the contracts a childless element still owns.
    ///
    /// # Panics
    ///
    /// Panics if the element violates one of those contracts.
    pub fn leaf<W>(&self, make: impl Fn() -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
    {
        let mut tester = ElementTester::mount(make());

        tester.diagnostics();

        let handle = tester.root_handle();
        tester.dispatch(handle, Box::new(()));
    }

    /// Checks a single-child element across mount, an in-place rebuild, a routed message, and the
    /// `describe` recursion that reaches its child.
    ///
    /// # Panics
    ///
    /// Panics if the element violates a lifecycle or routing contract.
    pub fn single_child<W>(&self, make: impl Fn(Tracked) -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
    {
        let ledger = Ledger::default();

        let mut tester = ElementTester::mount(make(ledger.tracked(1, None)));
        assert_eq!(
            ledger.mounts(1),
            1,
            "the child mounts when the tree is mounted"
        );
        assert_eq!(ledger.unmounts(1), 0);

        tester.rebuild(make(ledger.tracked(1, None)));
        assert_eq!(
            ledger.mounts(1),
            1,
            "an in-place rebuild reuses the child, it does not remount it"
        );
        assert_eq!(ledger.unmounts(1), 0, "an in-place rebuild keeps the child");

        tester.diagnostics();
        assert_eq!(
            ledger.describes(1),
            1,
            "describe must recurse into the child, not stop at the element"
        );

        tester.dispatch(ledger.handle(1), Box::new(()));
        assert_eq!(ledger.hits(1), 1, "a routed message reaches the child");
    }

    /// Checks a multi-child element across growth, truncation, clearing, a keyed reorder, keyed removal,
    /// and the address stability that keeps a captured dispatch reaching its child.
    ///
    /// # Panics
    ///
    /// Panics if the element violates a lifecycle or routing contract.
    pub fn multi_child<W>(&self, make: impl Fn(Vec<Tracked>) -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
    {
        self.check_positional(&make);
        self.check_keyed(&make);
    }

    /// Tail-only growth and truncation, where each position keeps a stable id, so mount and unmount
    /// counts read directly.
    fn check_positional<W>(&self, make: &impl Fn(Vec<Tracked>) -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
    {
        let ledger = Ledger::default();

        let unkeyed = |ids: &[u32]| ids.iter().map(|&id| ledger.tracked(id, None)).collect();

        let mut tester = ElementTester::mount(make(unkeyed(&[1, 2, 3])));
        for id in [1, 2, 3] {
            assert_eq!(ledger.mounts(id), 1, "each initial child mounts once");
            assert_eq!(ledger.unmounts(id), 0);
        }

        tester.diagnostics();
        for id in [1, 2, 3] {
            assert_eq!(
                ledger.describes(id),
                1,
                "describe must recurse into every child, not stop at the element"
            );
        }

        tester.rebuild(make(unkeyed(&[1, 2, 3, 4])));
        assert_eq!(ledger.mounts(4), 1, "the appended child mounts");
        for id in [1, 2, 3] {
            assert_eq!(ledger.mounts(id), 1, "surviving children are not remounted");
            assert_eq!(ledger.unmounts(id), 0);
        }

        tester.rebuild(make(unkeyed(&[1, 2])));
        for id in [3, 4] {
            assert_eq!(ledger.unmounts(id), 1, "a truncated child unmounts once");
        }
        for id in [1, 2] {
            assert_eq!(ledger.unmounts(id), 0, "survivors stay mounted");
        }

        tester.rebuild(make(unkeyed(&[])));
        for id in [1, 2] {
            assert_eq!(ledger.unmounts(id), 1, "clearing unmounts the rest");
        }

        assert_balanced(&ledger);
    }

    /// A keyed reorder and removal, where identity follows the key rather than the position.
    fn check_keyed<W>(&self, make: &impl Fn(Vec<Tracked>) -> W)
    where
        W: Widget + 'static,
        W::Element: 'static,
    {
        let ledger = Ledger::default();

        let keyed = |ids: &[u32]| ids.iter().map(|&id| ledger.tracked(id, Some(id))).collect();

        let mut tester = ElementTester::mount(make(keyed(&[1, 2, 3])));

        // The address of key 1, captured before the reorder moves it.
        let one = ledger.handle(1);

        tester.rebuild(make(keyed(&[3, 1, 2])));
        for id in [1, 2, 3] {
            assert_eq!(
                ledger.mounts(id),
                1,
                "a reorder reuses every child, none remount"
            );
            assert_eq!(ledger.unmounts(id), 0, "a reorder unmounts nothing");
        }

        tester.dispatch(one, Box::new(()));
        assert_eq!(
            ledger.hits(1),
            1,
            "the address is stable, so the captured handle still reaches key 1 after the reorder"
        );

        tester.rebuild(make(keyed(&[1, 3])));
        assert_eq!(ledger.unmounts(2), 1, "the removed key unmounts once");
        assert_eq!(ledger.unmounts(1), 0, "the kept keys stay mounted");
        assert_eq!(ledger.unmounts(3), 0);

        tester.dispatch(ledger.handle(2), Box::new(()));
        assert_eq!(
            ledger.hits(2),
            0,
            "a dispatch to the removed key is dropped, not misrouted onto a sibling"
        );

        tester.rebuild(make(keyed(&[])));
        assert_balanced(&ledger);
    }
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
    use agui::{
        prelude::{element::*, render_object::RenderObjectPtr},
        widgets::{flex::Column, sized_box::SizedBox},
    };

    use super::{ElementLifecycleCheck, Tracked};

    #[test]
    fn a_real_single_child_widget_satisfies_the_contract() {
        ElementLifecycleCheck::new().single_child(|child| SizedBox::new().child(child));
    }

    #[test]
    fn a_real_multi_child_widget_satisfies_the_contract() {
        ElementLifecycleCheck::new()
            .multi_child(|children| Column::builder().children(children).build());
    }

    #[test]
    fn a_childless_widget_survives_the_lifecycle() {
        ElementLifecycleCheck::new().leaf(SizedBox::new);
    }

    /// A multi-child container, built on the real [`Column`], whose element's `describe` stops at itself
    /// instead of recursing into its children, so the checker's self-test confirms it catches a container
    /// that hides its children.
    struct DescribeStops {
        children: Vec<Tracked>,
    }

    /// The element of [`DescribeStops`]: the real `Column` element with only `describe` sabotaged.
    struct DescribeStopsElement {
        inner: <Column<Vec<Tracked>> as Widget>::Element,
    }

    // SAFETY: every child-management and render-resolution obligation forwards to the wrapped `Column`
    // element, which upholds the lifecycle contract. Only `describe`, which carries no safety obligation,
    // is sabotaged.
    unsafe impl Element for DescribeStopsElement {
        type Render = <Column<Vec<Tracked>> as Widget>::Render;

        fn render_object_ptr(&self) -> RenderObjectPtr<Self::Render> {
            self.inner.render_object_ptr()
        }

        fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
            self.inner.mount(ctx);
        }

        fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
            self.inner.unmount(ctx);
        }

        fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
            d.node_for::<Self>().finish()
        }
    }

    impl Widget for DescribeStops {
        type Element = DescribeStopsElement;

        type Render = <Column<Vec<Tracked>> as Widget>::Render;

        fn create(self, ctx: &mut CreateCtx) -> DescribeStopsElement {
            let column = Column::builder().children(self.children).build();

            DescribeStopsElement {
                inner: column.create(ctx),
            }
        }

        fn update(self, ctx: &mut UpdateCtx<'_>, element: &mut DescribeStopsElement) {
            let column = Column::builder().children(self.children).build();

            Widget::update(column, ctx, &mut element.inner);
        }
    }

    #[test]
    #[should_panic(expected = "recurse")]
    fn a_container_that_hides_its_children_from_describe_is_caught() {
        ElementLifecycleCheck::new().multi_child(|children| DescribeStops { children });
    }
}
