use std::time::Duration;

use agui_core::{
    paint::scene::Scene,
    pipeline::PipelineOwner,
    prelude::{
        element::*,
        render_object::{BoxConstraints, HitTestResult, RenderBox},
    },
    scheduling::Vsync,
    test_harness::TestCtx,
    view::ViewHandle,
};

use crate::gesture::{PointerDispatcher, PointerEvent, PointerEventKind, PointerId, TestGesture};

/// A harness for testing a widget tree.
///
/// Build one with [`mount`](Self::mount), give it a surface size with [`resize`](Self::resize), then
/// advance it with [`pump`](Self::pump). Read back the result through a [`Probe`](crate::Probe) placed
/// in the tree or through the composited [`scene`](Self::scene), and drive pointer input by coordinate
/// with [`tap_at`](Self::tap_at), [`drag_from`](Self::drag_from), and
/// [`start_gesture`](Self::start_gesture).
pub struct WidgetTester {
    owner: PipelineOwner,
    view: ViewHandle,

    ctx: TestCtx,

    vsync: Vsync,
    now: Duration,

    dispatcher: PointerDispatcher,
    next_pointer: u64,
}

impl WidgetTester {
    /// Mounts `widget` as the root of a fresh tree, ready to be sized and pumped.
    pub fn mount<V>(widget: V) -> Self
    where
        V: Widget + 'static,
        V::Render: RenderBox + 'static,
    {
        let mut ctx = TestCtx::new();

        let (owner, view) = ctx.mount_view(widget);

        Self {
            owner,
            view,

            ctx,

            vsync: Vsync::new(),
            now: Duration::ZERO,

            dispatcher: PointerDispatcher::new(),
            next_pointer: 0,
        }
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
        for (path, message) in messages {
            self.owner.dispatch_message(&path, message);
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

    /// The scene composited from the most recent paint.
    pub fn scene(&self) -> Scene {
        self.view.composite_frame().rasterize()
    }

    /// Captures the element tree as a diagnostics snapshot.
    pub fn element_diagnostics(&self) -> DiagnosticsNode {
        self.owner.diagnostics()
    }

    /// Captures the render tree as a diagnostics snapshot.
    pub fn render_diagnostics(&self) -> DiagnosticsNode {
        self.view.diagnostics()
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
    pub fn start_gesture(&mut self, position: Offset) -> TestGesture<'_> {
        let pointer = self.allocate_pointer();
        self.send_pointer(pointer, PointerEventKind::Down, position);

        TestGesture::new(self, pointer, position)
    }

    /// Dispatches `message` to the element at `path`, marking it to rebuild on the next pump if it asks
    /// to.
    pub fn send<M: 'static>(&mut self, path: &[RoutingId], message: M) {
        let bytes = RoutingId::encode_path(path.iter().copied());
        let target = RoutingTarget::new(self.owner.root_id(), bytes);
        self.owner.dispatch_message(&target, Box::new(message));
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
