use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use agui_core::tree::{NodeHandle, Slot, Tree};

use crate::{
    context::{CreateCtx, LayoutCtx, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{Element, LeafElement},
    pipeline::{
        FramePhase, PipelineOwner,
        build_tree::{Build, BuildQueue, Operation, run},
        render_pipeline::{LayoutScope, RenderPipeline},
    },
    provide::ProvideScope,
    render_object::{
        box_layout::{BoxConstraints, RenderBox},
        node::RenderObjectPtr,
    },
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
    provide: ProvideScope,
    pipeline: RenderPipeline,
}

impl TestCtx {
    pub fn new() -> Self {
        Self {
            tasks: TestTaskRunner::new(),
            provide: ProvideScope::default(),
            pipeline: RenderPipeline::default(),
        }
    }

    /// Builds `widget`'s element and render object. A provided value the widget reads must come from a
    /// `Provide` wrapping it, since this context seeds no scope of its own.
    pub fn create<W: Widget>(&mut self, widget: W) -> W::Element {
        widget.create(&mut CreateCtx::new(self.provide, self.pipeline.clone()))
    }

    /// A [`LayoutCtx`] over this context's pipeline with a detached relayout scope, for laying a
    /// render object out in isolation. Reuse it to lay the same render object out more than once.
    pub fn layout_ctx(&self) -> LayoutCtx<'_, '_> {
        LayoutCtx::new(&self.pipeline, LayoutScope::detached())
    }

    /// Builds `widget`'s element and lays its render object out once under `constraints`, returning the
    /// element that owns the laid-out render object. For a render object laid out repeatedly, build it with
    /// [`create`](Self::create) and drive [`layout_ctx`](Self::layout_ctx) directly.
    pub fn laid_out<W>(&mut self, widget: W, constraints: BoxConstraints) -> W::Element
    where
        W: Widget,
        W::Render: RenderBox,
    {
        let mut element = self.create(widget);
        element
            .render_object_mut()
            .layout(&mut self.layout_ctx(), constraints);

        element
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

/// A leaf widget that hands a pre-built render object to the pipeline unchanged, so a test can drive a
/// hand-built render subtree that has no widget of its own. Wrap it in a [`View`] through
/// [`TestCtx::mount_view`] to lay it out, paint it, and hit-test it.
pub struct RawWidget<R> {
    render: R,
}

impl<R> RawWidget<R> {
    pub fn new(render: R) -> Self {
        Self { render }
    }
}

impl<R> Widget for RawWidget<R>
where
    R: RenderBox,
{
    type Element = LeafElement<R>;

    type Render = R;

    fn create(self, _: &mut CreateCtx) -> Self::Element {
        LeafElement::new(self.render)
    }

    fn update(self, _: &mut UpdateCtx, _: &mut Self::Element) {}
}

/// Drives one widget through the build phase for tests: it mounts the widget under a render-less root,
/// reconciles it against a new widget on [`rebuild`](Self::rebuild), and delivers messages by handle. It does
/// not lay out or paint; it exercises element mount, reconcile, and dispatch.
pub struct WidgetTester<W: Widget> {
    tree: Tree<TestRoot<W>, Build>,
    queue: BuildQueue,
    provide: ProvideScope,
    pipeline: RenderPipeline,
    tasks: TestTaskRunner,
}

impl<W> WidgetTester<W>
where
    W: Widget + 'static,
    W::Element: 'static,
{
    /// Mounts `widget` as the sole child of a fresh root.
    pub fn mount(widget: W) -> Self {
        let provide = ProvideScope::default();
        let mut queue = BuildQueue::new();
        let pipeline = RenderPipeline::default();
        let mut tasks = TestTaskRunner::new();

        let child = widget.create(&mut CreateCtx::new(provide, pipeline.clone()));
        let root = TestRoot {
            child: Slot::new(child),
            pending: None,
            render: (),
        };

        let tree = {
            let mut scheduler = tasks.scheduler();
            Tree::<TestRoot<W>, Build>::new(root, run::<TestRoot<W>>, |root, cursor| {
                root.mount(&mut UpdateCtx::new(
                    cursor,
                    provide,
                    &mut queue,
                    &pipeline,
                    &mut scheduler,
                ));
            })
        };

        Self {
            tree,
            queue,
            provide,
            pipeline,
            tasks,
        }
    }

    /// Reconciles the mounted widget against `widget`, then flushes any rebuilds it triggers.
    pub fn rebuild(&mut self, widget: W) {
        let root = self.tree.root_handle();
        // The root reconciles its child against the widget it holds, so hand the new one over before marking
        // the root to rebuild.
        // SAFETY: the root node is a `TestRoot<W>`.
        unsafe {
            self.tree
                .with_node::<TestRoot<W>, _>(root, |root| root.pending = Some(widget));
        }
        self.queue.mark_rebuild(root);
        self.flush();
    }

    /// Delivers `message` to the element at `handle`, then flushes any rebuilds it triggers. A message to a
    /// handle whose element is gone is dropped.
    pub fn dispatch(&mut self, handle: NodeHandle, message: Box<dyn Any>) {
        {
            let Self { tree, queue, .. } = self;
            tree.dispatch(
                handle,
                Operation::Message(MessageCtx::new(message, handle, queue)),
            );
        }
        self.flush();
    }

    /// Runs every spawned task to completion, then delivers the messages they posted back to their elements,
    /// flushing the rebuilds those messages trigger.
    pub fn run_tasks(&mut self) {
        self.tasks.run_to_completion();
        let posted: Vec<TaskEventMessage> = self.tasks.messages().collect();
        for (handle, message) in posted {
            self.dispatch(handle, message);
        }
    }

    /// The mounted widget's element, for inspecting the reconciled tree.
    pub fn root(&self) -> &W::Element {
        self.tree.root().child.get()
    }

    /// The handle of the mounted widget's element, for dispatching a message to it.
    pub fn root_handle(&self) -> NodeHandle {
        self.tree.root().child.handle()
    }

    /// A diagnostics snapshot of the mounted subtree.
    pub fn diagnostics(&self) -> DiagnosticsNode {
        self.tree.root().describe(&mut Diagnostics::new())
    }

    /// Drains every element marked to rebuild, shallowest-first, as a frame's build flush does.
    fn flush(&mut self) {
        let Self {
            tree,
            queue,
            provide,
            pipeline,
            tasks,
        } = self;
        let pipeline: &RenderPipeline = pipeline;
        let mut scheduler = tasks.scheduler();

        let _phase = pipeline.enter_phase(FramePhase::Build);

        while let Some((handle, is_dependency_change)) = queue.take_shallowest(tree) {
            tree.dispatch_with_cursor(handle, |cursor| {
                let ctx = UpdateCtx::new(cursor, *provide, queue, pipeline, &mut scheduler);
                if is_dependency_change {
                    Operation::DependencyChanged(ctx)
                } else {
                    Operation::Rebuild(ctx)
                }
            });
        }
    }
}

/// The render-less root [`WidgetTester`] mounts the widget-under-test under. It holds that widget as its one
/// child and reconciles it with a new widget on rebuild, standing in for the parent that would otherwise
/// drive the reconcile.
struct TestRoot<W: Widget> {
    child: Slot<W::Element>,
    pending: Option<W>,
    render: (),
}

// SAFETY: mounts and unmounts its single child through the cursor child operations; its own render is `()` and
// never dereferenced.
unsafe impl<W> Element for TestRoot<W>
where
    W: Widget + 'static,
    W::Element: 'static,
{
    type Render = ();

    fn render_object_mut(&mut self) -> &mut () {
        &mut self.render
    }

    fn render_object_ptr(&self) -> RenderObjectPtr<()> {
        RenderObjectPtr::dangling()
    }

    fn mount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // The tester does not lay out, so the child's render has no parent pointer to wire into; mounting it
        // for its own subtree is all that is needed, and the returned handle is dropped.
        // SAFETY: `self.child` is our own slot.
        let _mounted = unsafe { ctx.mount(&mut self.child) };
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(widget) = self.pending.take() {
            // SAFETY: `self.child` is our own slot.
            unsafe { ctx.with_child(&mut self.child, |element, ctx| widget.update(ctx, element)) };
        }
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.child.get().describe(d)
    }
}

/// A reconcilable, render-bearing root for a full-frame harness. It builds an initial widget as its child and
/// forwards its render object to it, then reconciles that child against a new widget delivered as a message,
/// so the harness can re-drive the whole tree with a fresh root widget. It captures its own handle at mount so
/// the harness can address it for that message.
pub struct HarnessRoot<W> {
    initial: W,
    handle: Rc<Cell<Option<NodeHandle>>>,
}

impl<W> HarnessRoot<W> {
    pub fn new(initial: W, handle: Rc<Cell<Option<NodeHandle>>>) -> Self {
        Self { initial, handle }
    }
}

/// The element of a [`HarnessRoot`], holding the mounted child and the widget queued for the next rebuild.
pub struct HarnessRootElement<W: Widget> {
    child: Slot<W::Element>,
    pending: Option<W>,
    handle: Rc<Cell<Option<NodeHandle>>>,
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

        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.mount(&mut self.child) };
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx<'_>) {
        // SAFETY: `self.child` is our own slot.
        unsafe { ctx.unmount(&mut self.child) };
    }

    fn rebuild(&mut self, ctx: &mut UpdateCtx<'_>) {
        if let Some(widget) = self.pending.take() {
            // SAFETY: `self.child` is our own slot.
            unsafe { ctx.with_child(&mut self.child, |element, ctx| widget.update(ctx, element)) };
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

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use super::WidgetTester;
    use crate::scheduling::TaskHandle;
    use crate::test_fixtures::Leaf;

    #[test]
    fn a_spawned_task_posts_a_message_back_to_its_element() {
        let received = Rc::new(Cell::new(None));
        let recorder = Rc::clone(&received);

        let widget = Leaf::new()
            .on_mount({
                // The element holds the task handle for its own lifetime, as a real element would; dropping
                // it would cancel the task.
                let handle: RefCell<Option<TaskHandle>> = RefCell::new(None);
                move |ctx| {
                    *handle.borrow_mut() = Some(
                        ctx.spawn(|task| async move { task.send(42u32) })
                            .expect("scheduler available during mount"),
                    );
                }
            })
            .on_message(move |ctx| recorder.set(Some(ctx.consume::<u32>())));

        let mut tester = WidgetTester::mount(widget);
        assert_eq!(received.get(), None);

        tester.run_tasks();
        assert_eq!(received.get(), Some(42));
    }
}
