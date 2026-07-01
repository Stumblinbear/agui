use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

use crate::test_harness::{HarnessRoot, TestCtx};
use agui::{
    paint::compositing::CompositedFrame,
    pipeline::PipelineOwner,
    prelude::{
        element::*,
        render_object::{BoxConstraints, HitTestResult, RenderBox},
    },
    scheduling::Vsync,
    semantics::SemanticsTree,
    view::ViewHandle,
};

use crate::gesture::{PointerDispatcher, PointerEvent, PointerEventKind, PointerId, TestGesture};

/// A harness for testing a widget tree rooted at a `W`.
///
/// Build one with [`mount`](Self::mount), give it a surface size with [`resize`](Self::resize), then
/// advance it with [`pump`](Self::pump). Re-drive the whole tree against a new root widget with
/// [`rebuild`](Self::rebuild). Read back the result through a [`Probe`](crate::Probe) placed in the tree or
/// through the composited [`scene`](Self::scene), and drive pointer input by coordinate with
/// [`tap_at`](Self::tap_at), [`drag_from`](Self::drag_from), and [`start_gesture`](Self::start_gesture).
pub struct WidgetTester<W: Widget> {
    owner: PipelineOwner,
    view: ViewHandle,

    ctx: TestCtx,

    vsync: Vsync,
    now: Duration,

    dispatcher: PointerDispatcher,
    next_pointer: u64,

    /// The reconcilable root's handle, captured at mount, for re-driving it on [`rebuild`](Self::rebuild).
    root: Rc<Cell<Option<NodeHandle>>>,
    _root: PhantomData<fn() -> W>,
}

impl<W> WidgetTester<W>
where
    W: Widget + 'static,
    W::Element: 'static,
    W::Render: RenderBox + Sized + 'static,
{
    /// Mounts `widget` as the root of a fresh tree, ready to be sized and pumped.
    pub fn mount(widget: W) -> Self {
        let mut ctx = TestCtx::new();

        let root = Rc::new(Cell::new(None));
        let (owner, view) = ctx.mount_view(HarnessRoot::new(widget, Rc::clone(&root)));

        Self {
            owner,
            view,

            ctx,

            vsync: Vsync::new(),
            now: Duration::ZERO,

            dispatcher: PointerDispatcher::new(),
            next_pointer: 0,

            root,
            _root: PhantomData,
        }
    }

    /// Reconciles the tree against `widget` as a fresh root, then produces a frame, so a test can observe how
    /// the tree responds to a rebuild.
    ///
    /// # Panics
    ///
    /// Panics if the root has not mounted, which never happens for a tester built by [`mount`](Self::mount).
    pub fn rebuild(&mut self, widget: W) {
        let handle = self
            .root
            .get()
            .expect("the root captured its handle at mount");
        self.owner.dispatch_message(handle, Box::new(widget));
        self.pump(Duration::ZERO);
    }

    /// Lays the root out as tightly constrained to `size`, repainting it on the next pump.
    pub fn resize(&mut self, size: Size) {
        self.view.resize(BoxConstraints::tight(size));
    }

    /// Lays the root out under `constraints`, repainting it on the next pump.
    pub fn resize_with(&mut self, constraints: BoxConstraints) {
        self.view.resize(constraints);
    }

    /// Advances time by `delta` and produces one frame: it ticks frame callbacks, runs spawned tasks
    /// one step, delivers the messages they posted, rebuilds what was dirtied, and lays out and paints
    /// what changed.
    pub fn pump(&mut self, delta: Duration) {
        self.now += delta;
        self.vsync.tick(self.now);

        self.ctx.poll();

        let messages = self.ctx.messages().collect::<Vec<_>>();
        for (handle, message) in messages {
            self.owner.dispatch_message(handle, message);
        }

        self.owner.flush_build(&mut self.ctx.scheduler());

        self.owner.flush_layout();
        self.owner.flush_paint();
    }

    /// Pumps a 16ms frame repeatedly until no frame callback is registered and nothing is left to
    /// rebuild.
    ///
    /// # Panics
    ///
    /// Panics if the tree has not settled within `max_frames`, which usually means an animation never
    /// ends.
    pub fn pump_and_settle(&mut self, max_frames: usize) {
        for _ in 0..max_frames {
            self.pump(Duration::from_millis(16));

            if self.vsync.is_idle() && !self.owner.is_dirty() {
                return;
            }
        }

        panic!("the tree did not settle within {max_frames} frames");
    }

    /// Polls every spawned task once without producing a frame. A message a task posts is delivered on
    /// the next [`pump`](Self::pump).
    pub fn poll_tasks(&mut self) {
        self.ctx.poll();
    }

    /// Runs every spawned task to completion. Use only for tasks that finish on their own; a long-lived
    /// task never returns and this spins forever.
    pub fn run_tasks_to_completion(&mut self) {
        self.ctx.run_tasks_to_completion();
    }

    /// Re-walks every semantics boundary marked since the last flush, the work a driver does to refresh
    /// assistive technology after a change. A boundary left unmarked is not re-walked, so a render object's
    /// semantics-walk count reveals which boundaries a change actually touched.
    pub fn flush_semantics(&mut self) {
        self.owner.flush_semantics();
    }

    /// The frame composited from the most recent paint, before rasterization.
    pub fn composite_frame(&self) -> CompositedFrame {
        self.view.composite_frame()
    }

    /// Captures the element tree as a diagnostics snapshot.
    pub fn element_diagnostics(&self) -> DiagnosticsNode {
        self.owner.diagnostics()
    }

    /// Captures the render tree as a diagnostics snapshot.
    pub fn render_diagnostics(&self) -> DiagnosticsNode {
        self.view.diagnostics()
    }

    /// Captures the semantics tree the view exposes to assistive technology, for finding nodes by name or
    /// role.
    pub fn semantics(&self) -> SemanticsTree {
        self.view.semantics()
    }

    /// Hit-tests the tree at `position`, in the root coordinate space, returning the handlers under it
    /// ordered most-specific first.
    pub fn hit_test(&self, position: Offset) -> HitTestResult {
        self.view.hit_test(position)
    }

    /// Presses and releases a pointer at `position`.
    pub fn tap_at(&mut self, position: Offset) {
        let pointer = self.allocate_pointer();
        self.send_pointer(pointer, PointerEventKind::Down, position);
        self.send_pointer(pointer, PointerEventKind::Up, position);
    }

    /// Presses a pointer at `start`, moves it by `delta`, and releases it.
    pub fn drag_from(&mut self, start: Offset, delta: Offset) {
        let end = start + delta;
        let pointer = self.allocate_pointer();
        self.send_pointer(pointer, PointerEventKind::Down, start);
        self.send_pointer(pointer, PointerEventKind::Move, end);
        self.send_pointer(pointer, PointerEventKind::Up, end);
    }

    /// Presses a pointer at `position` and returns a gesture that moves and releases it step by step.
    pub fn start_gesture(&mut self, position: Offset) -> TestGesture<'_, W> {
        let pointer = self.allocate_pointer();
        self.send_pointer(pointer, PointerEventKind::Down, position);

        TestGesture::new(self, pointer, position)
    }

    /// Dispatches `message` to the element at `handle`, marking it to rebuild on the next pump if it asks
    /// to. A message addressed to a handle whose element is gone is dropped.
    pub fn send<M: 'static>(&mut self, handle: NodeHandle, message: M) {
        self.owner.dispatch_message(handle, Box::new(message));
    }

    /// A clone of the frame-callback registry the tester ticks each pump, for handing to a render
    /// object that animates.
    pub fn vsync(&self) -> Vsync {
        self.vsync.clone()
    }

    fn allocate_pointer(&mut self) -> PointerId {
        let id = PointerId(self.next_pointer);
        self.next_pointer += 1;
        id
    }

    pub(crate) fn send_pointer(
        &mut self,
        pointer: PointerId,
        kind: PointerEventKind,
        position: Offset,
    ) {
        let event = PointerEvent {
            pointer,
            position,
            kind,
        };

        self.dispatcher
            .handle(&event, |position| self.view.hit_test(position));
    }
}
