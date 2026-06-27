use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use agui_core::tree::{NodeHandle, Slot};

use agui::{
    context::{CreateCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::Element,
    pipeline::{PipelineOwner, render_pipeline::SemanticsScope},
    render_object::{box_layout::RenderBox, node::RenderObjectPtr},
    scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler},
    view::{View, ViewHandle},
    widget::Widget,
};

enum TaskRunnerEvent {
    Dropped(usize),
}

type DeferredTask = (TaskFuture, Rc<Cell<bool>>);

/// Test scheduler that drives spawned tasks to completion synchronously, so a test can mount and
/// then drain the event channel deterministically.
pub struct TestTaskRunner {
    task_event_rx: mpsc::Receiver<TaskEventMessage>,
    task_event_tx: EventSender,

    spawned_tasks: Vec<Option<TaskFuture>>,

    runner_event_rx: mpsc::Receiver<TaskRunnerEvent>,
    runner_event_tx: mpsc::Sender<TaskRunnerEvent>,

    deferred_rx: mpsc::Receiver<DeferredTask>,
    deferred_tx: mpsc::Sender<DeferredTask>,
    deferred_tasks: Vec<DeferredTask>,
}

impl TestTaskRunner {
    pub fn new() -> Self {
        let (task_event_tx, task_event_rx) = mpsc::channel();
        let (runner_event_tx, runner_event_rx) = mpsc::channel();
        let (deferred_tx, deferred_rx) = mpsc::channel();

        Self {
            task_event_rx,
            task_event_tx,

            spawned_tasks: Vec::new(),

            runner_event_rx,
            runner_event_tx,

            deferred_rx,
            deferred_tx,
            deferred_tasks: Vec::new(),
        }
    }

    pub fn scheduler(&mut self) -> TestTaskScheduler<'_> {
        TestTaskScheduler {
            task_event_tx: self.task_event_tx.clone(),
            spawned_tasks: &mut self.spawned_tasks,
            runner_event_tx: &self.runner_event_tx,
            deferred_tx: self.deferred_tx.clone(),
        }
    }

    pub fn messages(&self) -> impl Iterator<Item = TaskEventMessage> + '_ {
        self.task_event_rx.try_iter()
    }

    /// Poll every live spawned task once, dropping any that complete or whose handle was dropped.
    pub fn poll(&mut self) {
        self.reap_dropped();

        while let Ok(task) = self.deferred_rx.try_recv() {
            self.deferred_tasks.push(task);
        }

        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);

        for slot in &mut self.spawned_tasks {
            if let Some(future) = slot.as_mut()
                && future.as_mut().poll(&mut cx).is_ready()
            {
                *slot = None;
            }
        }

        self.deferred_tasks.retain_mut(|(future, cancelled)| {
            !cancelled.get() && future.as_mut().poll(&mut cx).is_pending()
        });
    }

    /// Poll repeatedly until no live task remains. A task that never completes will spin forever;
    /// tests are expected to spawn tasks that finish.
    pub fn run_to_completion(&mut self) {
        while {
            self.poll();
            self.spawned_tasks.iter().any(Option::is_some) || !self.deferred_tasks.is_empty()
        } {}
    }

    /// Drop the futures of any tasks whose [`TaskHandle`] has been dropped.
    fn reap_dropped(&mut self) {
        while let Ok(TaskRunnerEvent::Dropped(id)) = self.runner_event_rx.try_recv() {
            if let Some(slot) = self.spawned_tasks.get_mut(id) {
                *slot = None;
            }
        }
    }
}

impl Default for TestTaskRunner {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TestTaskScheduler<'a> {
    task_event_tx: EventSender,

    spawned_tasks: &'a mut Vec<Option<TaskFuture>>,
    runner_event_tx: &'a mpsc::Sender<TaskRunnerEvent>,
    deferred_tx: mpsc::Sender<DeferredTask>,
}

impl TaskScheduler for TestTaskScheduler<'_> {
    fn event_tx(&self) -> EventSender {
        self.task_event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        let id = self.spawned_tasks.len();

        self.spawned_tasks.push(Some(func));

        let runner_event_tx = self.runner_event_tx.clone();

        Ok(TaskHandle::new(Box::new(move || {
            // The runner may already be gone at teardown; a failed send just means there is nothing to reap.
            let _ = runner_event_tx.send(TaskRunnerEvent::Dropped(id));
        })))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(DeferredTestTaskScheduler {
            task_event_tx: self.task_event_tx.clone(),
            deferred_tx: self.deferred_tx.clone(),
        })
    }
}

/// Owned scheduler handle into a [`TestTaskRunner`]. Free of borrows, so it can be captured by a
/// `LayoutBuilder` and used to spawn during layout. Spawning sends over a channel the runner drains;
/// cancellation rides a flag the [`TaskHandle`] flips on drop.
pub struct DeferredTestTaskScheduler {
    task_event_tx: EventSender,
    deferred_tx: mpsc::Sender<DeferredTask>,
}

impl TaskScheduler for DeferredTestTaskScheduler {
    fn event_tx(&self) -> EventSender {
        self.task_event_tx.clone()
    }

    fn spawn(&mut self, func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        let cancelled = Rc::new(Cell::new(false));

        if self
            .deferred_tx
            .send((func, Rc::clone(&cancelled)))
            .is_err()
        {
            return Err(String::from("the task runner has been dropped").into());
        }

        Ok(TaskHandle::new(Box::new(move || {
            cancelled.set(true);
        })))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(DeferredTestTaskScheduler {
            task_event_tx: self.task_event_tx.clone(),
            deferred_tx: self.deferred_tx.clone(),
        })
    }
}

fn noop_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(std::ptr::null(), &VTABLE)
    }

    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

    // SAFETY: every vtable entry is a no-op over the null data pointer, so the waker upholds the
    // `RawWaker` contract trivially.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

pub struct TestCtx {
    tasks: TestTaskRunner,
}

impl TestCtx {
    pub fn new() -> Self {
        Self {
            tasks: TestTaskRunner::new(),
        }
    }

    /// Mounts `widget` as the child of a [`View`] and returns the owner together with the view's handle.
    ///
    /// # Panics
    ///
    /// Panics if the [`View`] does not fill its surface slot at mount, which never happens for a mounted
    /// view.
    pub fn mount_view<V>(&mut self, widget: V) -> (PipelineOwner, ViewHandle)
    where
        V: Widget + 'static,
        V::Element: 'static,
        V::Render: RenderBox + Sized,
    {
        let surface = Rc::new(RefCell::new(None));

        let owner = PipelineOwner::new(
            View::new(Rc::clone(&surface)).child(widget),
            &mut self.tasks.scheduler(),
        );

        let view = surface
            .borrow()
            .clone()
            .expect("a mounted View fills its surface slot");

        (owner, view)
    }

    /// A scheduler handle into this context's task runner, for driving an owner's `flush_build`.
    pub fn scheduler(&mut self) -> TestTaskScheduler<'_> {
        self.tasks.scheduler()
    }

    /// Polls every spawned task once. A message a task posts is read back through
    /// [`messages`](Self::messages).
    pub fn poll(&mut self) {
        self.tasks.poll();
    }

    /// Runs every spawned task to completion. Use only for tasks that finish on their own.
    pub fn run_tasks_to_completion(&mut self) {
        self.tasks.run_to_completion();
    }

    /// The messages spawned tasks have posted since the last drain.
    pub fn messages(&self) -> impl Iterator<Item = TaskEventMessage> + '_ {
        self.tasks.messages()
    }
}

impl Default for TestCtx {
    fn default() -> Self {
        Self::new()
    }
}

/// A reconcilable, render-bearing root for a full-frame harness. It builds an initial widget as its child and
/// forwards its render object to it, then reconciles that child against a new widget delivered as a message,
/// so the harness can re-drive the whole tree with a fresh root widget. It captures its own handle at mount so
/// the harness can address it for that message.
pub(crate) struct HarnessRoot<W> {
    initial: W,
    handle: Rc<Cell<Option<NodeHandle>>>,
}

impl<W> HarnessRoot<W> {
    pub fn new(initial: W, handle: Rc<Cell<Option<NodeHandle>>>) -> Self {
        Self { initial, handle }
    }
}

/// The element of a [`HarnessRoot`], holding the mounted child and the widget queued for the next rebuild.
pub(crate) struct HarnessRootElement<W: Widget> {
    child: Slot<W::Element>,
    pending: Option<W>,
    handle: Rc<Cell<Option<NodeHandle>>>,
    /// The enclosing semantics boundary, captured at mount and re-entered on a rebuild. A directly-rebuilt
    /// element re-establishes it the way a stateful element does, since a rebuild dispatch arrives with a
    /// detached scope otherwise and a child's semantics mark would be dropped.
    semantics_scope: SemanticsScope,
}

impl<W> Widget for HarnessRoot<W>
where
    W: Widget + 'static,
    W::Element: 'static,
    W::Render: RenderBox + Sized + 'static,
{
    type Element = HarnessRootElement<W>;

    type Render = W::Render;

    fn create(self, ctx: &mut CreateCtx) -> HarnessRootElement<W> {
        HarnessRootElement {
            child: Slot::new(self.initial.create(ctx)),
            pending: None,
            handle: self.handle,
            semantics_scope: SemanticsScope::default(),
        }
    }

    fn update(self, _ctx: &mut UpdateCtx<'_>, _element: &mut HarnessRootElement<W>) {}
}

// SAFETY: manages its single child only through the cursor child operations, and forwards render resolution
// to it.
unsafe impl<W> Element for HarnessRootElement<W>
where
    W: Widget + 'static,
    W::Element: 'static,
    W::Render: RenderBox + Sized + 'static,
{
    type Render = W::Render;

    fn render_object_mut(&mut self) -> &mut W::Render {
        self.child.get_mut().render_object_mut()
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<W::Render> {
        self.child.get().render_object_ptr()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        self.handle.set(Some(ctx.handle()));
        self.semantics_scope = ctx.semantics_scope();

        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.mount(&mut self.child) };
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(widget) = self.pending.take() {
            ctx.with_semantics_scope(self.semantics_scope, |ctx| {
                // SAFETY: `self.child` is our own slot.
                unsafe {
                    ctx.with_child(&mut self.child, |element, ctx| widget.update(ctx, element));
                }
            });
        }
    }

    fn message(&mut self, ctx: &mut MessageCtx<'_>) {
        self.pending = Some(ctx.consume::<W>());
        ctx.request_rebuild();
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.get().describe(d)
    }
}
