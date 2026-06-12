use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

use crate::{
    context::{MessageCtx, UpdateCtx},
    element::{
        BuildBoundaryElement, BuildBoundaryId, BuildScope, BuildState, LeafElement, RoutingPath,
    },
    pipeline::{
        PipelineOwner,
        layout::LayoutPipeline,
        paint::{PaintPipeline, PaintScope},
    },
    provide::ProvideScope,
    render_object::{RenderObject, box_layout::RenderBox},
    scheduling::{EventSender, TaskEventMessage, TaskFuture, TaskHandle, TaskScheduler},
    view::{View, ViewHandle},
    widget::Widget,
};

/// A build-only owner over the build machinery, for tests that exercise rebuild dispatch without a
/// render pipeline. It builds a root widget as the outermost build boundary and drives marks and flushes
/// against it, threading a throwaway paint pipeline the build-only tests never lay out.
///
/// A test whose root render object is a [`RenderBox`](crate::render_object::box_layout::RenderBox)
/// drives a full [`PipelineOwner`](crate::pipeline::PipelineOwner) instead; this is for the tests whose
/// root has no box layout to lay out.
pub struct TestBuildOwner {
    state: Rc<RefCell<BuildState>>,
    root: BuildBoundaryElement,
}

impl TestBuildOwner {
    /// Builds `widget` as the root build boundary.
    pub fn mount<V>(widget: V, scheduler: &mut dyn TaskScheduler) -> Self
    where
        V: Widget,
        V::Element: 'static,
        V::Render: RenderObject,
    {
        let provide = ProvideScope::new();
        let (state, root_scope) = BuildState::new();

        let mut path = Vec::new();
        let mut paint = PaintPipeline::default();
        let layout = LayoutPipeline::default();
        let detached = PaintScope::detached();

        let mut ctx = UpdateCtx::new(
            scheduler,
            &mut path,
            &provide,
            &root_scope,
            &layout,
            &mut paint,
            &detached,
        );

        let (root, _render) = BuildBoundaryElement::create(widget, &mut ctx);

        Self { state, root }
    }

    /// The id of the root build boundary, for addressing a root-relative path.
    pub fn root_id(&self) -> BuildBoundaryId {
        self.root.id()
    }

    /// Whether any boundary is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        self.state.borrow().is_dirty()
    }

    /// Delivers `message` to the element at `path`, marking its boundary for rebuild if the element asks.
    pub fn dispatch_message(&mut self, path: &RoutingPath, message: Box<dyn Any>) {
        let mut ctx = MessageCtx::new(message);
        BuildState::deliver_message(&self.state, path, &mut ctx);
    }

    /// Marks the element at `path` to rebuild on the next [`flush`](Self::flush).
    pub fn request_rebuild(&mut self, path: &RoutingPath) {
        self.state.borrow_mut().mark_rebuild(path);
    }

    /// Marks the element at `path` to rebuild on the next [`flush`](Self::flush) because a provided value
    /// it depends on changed, so its dependency-change hook runs.
    pub fn request_dependency_change(&mut self, path: &RoutingPath) {
        self.state.borrow_mut().mark_dependency_changed(path);
    }

    /// Rebuilds every boundary marked since the last flush, returning whether anything rebuilt.
    pub fn flush(&mut self, scheduler: &mut dyn TaskScheduler) -> bool {
        let mut paint = PaintPipeline::default();
        let layout = LayoutPipeline::default();
        BuildState::flush(&self.state, scheduler, &layout, &mut paint)
    }
}

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
            runner_event_tx.send(TaskRunnerEvent::Dropped(id)).unwrap();
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

    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

/// A [`TaskScheduler`] that drops spawned work, for the tests that never poll tasks.
pub struct NoopScheduler {
    event_tx: EventSender,
}

impl NoopScheduler {
    pub fn new() -> Self {
        let (event_tx, _rx) = mpsc::channel();
        Self { event_tx }
    }
}

impl Default for NoopScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskScheduler for NoopScheduler {
    fn event_tx(&self) -> EventSender {
        self.event_tx.clone()
    }

    fn spawn(&mut self, _func: TaskFuture) -> Result<TaskHandle, Box<dyn std::error::Error>> {
        Ok(TaskHandle::new(Box::new(|| {})))
    }

    fn deferred(&self) -> Box<dyn TaskScheduler> {
        Box::new(NoopScheduler::new())
    }
}

/// Builds an [`UpdateCtx`] over a [`NoopScheduler`] and a detached scope, for driving a widget's
/// `create`/`update` (or a rebuild dispatch) directly in a test that does not poll tasks.
pub fn with_ctx<R>(f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
    with_ctx_in(&ProvideScope::new(), f)
}

/// Like [`with_ctx`], but over `provide_scope`, for tests that read a provided value.
pub fn with_ctx_in<R>(provide_scope: &ProvideScope, f: impl FnOnce(&mut UpdateCtx) -> R) -> R {
    let mut scheduler = NoopScheduler::new();
    let mut path = Vec::new();
    let mut paint = PaintPipeline::default();
    let layout = LayoutPipeline::default();

    f(&mut UpdateCtx::new(
        &mut scheduler,
        &mut path,
        provide_scope,
        &BuildScope::detached(),
        &layout,
        &mut paint,
        &PaintScope::detached(),
    ))
}

/// A leaf widget that hands a pre-built render object to the pipeline unchanged, so a test can drive a
/// hand-built render subtree that has no widget of its own. Wrap it in a [`View`] through
/// [`mount_view`] to lay it out, paint it, and hit-test it.
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
    R: RenderObject + RenderBox + 'static,
{
    type Element = LeafElement<R>;

    type Render = R;

    fn create(self, _: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        (LeafElement::new(), self.render)
    }

    fn update(self, _: &mut Self::Element, _: &mut Self::Render, _: &mut UpdateCtx) {}
}

/// Mounts `widget` as the child of a [`View`] and returns the owner together with the view's handle,
/// for tests that drive a render subtree's layout, paint, and hit-testing without polling tasks. A test
/// that polls the tasks its subtree spawns uses [`mount_view_with`] with its own scheduler instead.
pub fn mount_view<V>(widget: V) -> (PipelineOwner, ViewHandle)
where
    V: Widget,
    V::Element: 'static,
    V::Render: RenderObject + RenderBox,
{
    let mut scheduler = NoopScheduler::new();

    mount_view_with(widget, &mut scheduler)
}

/// Mounts `widget` as the child of a [`View`] over `scheduler` and returns the owner together with the
/// view's handle.
///
/// # Panics
///
/// Panics if the [`View`] does not fill its surface slot at mount, which never happens for a mounted
/// view.
pub fn mount_view_with<V>(
    widget: V,
    scheduler: &mut dyn TaskScheduler,
) -> (PipelineOwner, ViewHandle)
where
    V: Widget,
    V::Element: 'static,
    V::Render: RenderObject + RenderBox,
{
    let surface = Rc::new(RefCell::new(None));
    let owner = PipelineOwner::new(View::new(Rc::clone(&surface)).child(widget), scheduler);

    let view = surface
        .borrow()
        .clone()
        .expect("a mounted View fills its surface slot");

    (owner, view)
}
