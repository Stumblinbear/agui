use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    rc::{Rc, Weak},
};

use slotmap::SlotMap;

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::{AnyElement, Element, RoutingPath, RoutingTarget},
    pipeline::{
        layout::LayoutPipeline,
        paint::{PaintPipeline, PaintScope},
    },
    provide::ProvideScope,
    render_object::{AnyRenderObject, RenderObject},
    scheduling::TaskScheduler,
    widget::Widget,
};

type SharedRenderObject = Rc<RefCell<dyn AnyRenderObject>>;

/// Why a descendant was marked: a plain rebuild, or a change in a provided value it depends on. The
/// kind selects which dispatch the flush delivers, so a dependency change runs the element's
/// dependency-change hook.
#[derive(Clone, Copy)]
enum MarkKind {
    Rebuild,
    DependencyChanged,
}

slotmap::new_key_type! {
    /// Identifies a registered build boundary within one [`BuildState`].
    pub struct BuildBoundaryId;
}

/// The registry of build boundaries, shared between the owner that flushes them, the boundaries that mark
/// themselves, and the routing paths that address them.
///
/// Every element lives under exactly one boundary; the root is the outermost, registered like any other.
/// A boundary is addressed by its [`BoundaryId`], so a message or rebuild reaches it without walking from
/// the root.
pub struct BuildState {
    boundaries: SlotMap<BuildBoundaryId, Weak<BuildBoundaryCell>>,
    dirty: Vec<BuildBoundaryId>,
}

impl BuildState {
    /// A fresh registry, shared so boundaries can register and mark themselves into it.
    pub fn new() -> (Rc<RefCell<Self>>, BuildScope) {
        let state = Rc::new(RefCell::new(Self {
            boundaries: SlotMap::with_key(),
            dirty: Vec::new(),
        }));

        let root_scope = BuildScope::new_root(&state);

        (state, root_scope)
    }

    /// Whether any boundary is waiting to rebuild.
    pub fn is_dirty(&self) -> bool {
        !self.dirty.is_empty()
    }

    fn lookup(&self, id: BuildBoundaryId) -> Option<Rc<BuildBoundaryCell>> {
        self.boundaries.get(id).and_then(Weak::upgrade)
    }

    /// Marks the element at `target` to rebuild on the next flush.
    pub(crate) fn mark_rebuild(&mut self, target: &RoutingTarget) {
        self.mark(target, MarkKind::Rebuild);
    }

    /// Marks the element at `target` to rebuild on the next flush because a provided value it depends
    /// on changed, so its dependency-change hook runs.
    pub(crate) fn mark_dependency_changed(&mut self, target: &RoutingTarget) {
        self.mark(target, MarkKind::DependencyChanged);
    }

    /// Queues the boundary owning `target` for the next flush, recording the within-path and why. A
    /// target whose boundary is gone is dropped.
    fn mark(&mut self, target: &RoutingTarget, kind: MarkKind) {
        let Some(cell) = self
            .boundaries
            .get(target.boundary())
            .and_then(Weak::upgrade)
        else {
            return;
        };

        if !cell.is_dirty.get() {
            cell.is_dirty.set(true);
            self.dirty.push(cell.id.get());
        }

        cell.suffixes
            .borrow_mut()
            .push((target.path().as_bytes().into(), kind));
    }

    /// Drains the marked boundaries into their owning `Rc`s, ordered shallowest-depth first so that
    /// re-dispatching an outer boundary, which reconciles the boundaries nested in it, lets the inner ones
    /// be skipped rather than rebuilt twice.
    fn drain_rootmost_first(&mut self) -> Vec<Rc<BuildBoundaryCell>> {
        let ids: Vec<BuildBoundaryId> = self.dirty.drain(..).collect();

        let mut ordered: Vec<Rc<BuildBoundaryCell>> = ids
            .into_iter()
            .filter_map(|id| self.boundaries.get(id).and_then(Weak::upgrade))
            .collect();

        for cell in &ordered {
            cell.clear_dirty();
        }

        ordered.sort_by_key(|cell| cell.depth);

        ordered
    }

    /// Delivers `ctx` to the element at `path`, marking its boundary for rebuild if the element asks.
    /// A path whose boundary is gone (unmounted, or its inner replaced) is dropped.
    pub(crate) fn deliver_message(
        state: &Rc<RefCell<Self>>,
        target: &RoutingTarget,
        ctx: &mut MessageCtx,
    ) {
        let Some(cell) = state.borrow().lookup(target.boundary()) else {
            return;
        };

        cell.dispatch_within(target.path(), Dispatch::Message(ctx));

        if ctx.rebuild_requested() {
            state.borrow_mut().mark_rebuild(target);
        }
    }

    /// Rebuilds every boundary marked since the last flush. Returns whether anything rebuilt.
    ///
    /// A render object built during a rebuild is mounted into `paint` and `layout` under the nearest
    /// enclosing boundary, which the boundaries below the root re-establish as the walk descends.
    pub(crate) fn flush(
        state: &Rc<RefCell<Self>>,
        scheduler: &mut dyn TaskScheduler,
        layout: &LayoutPipeline,
        paint: &mut PaintPipeline,
    ) -> bool {
        let ordered = state.borrow_mut().drain_rootmost_first();

        if ordered.is_empty() {
            return false;
        }

        // Nothing encloses a rebuilt boundary above the root view, so each walk starts detached and the
        // views and repaint boundaries it crosses replace the scope as it descends.
        let detached = PaintScope::detached();

        for cell in ordered {
            cell.flush_rebuilds(scheduler, paint, layout, &detached);
        }

        true
    }
}

/// The enclosing build boundary a subtree is reconciled under, threaded through the build walk so a
/// boundary registers under it and the ids pushed below it address relative to it. A detached scope reaches
/// no registry, so a boundary registered under it is never marked or flushed.
#[derive(Clone)]
pub struct BuildScope {
    state: Weak<RefCell<BuildState>>,
    boundary: Option<BuildBoundaryId>,
    depth: usize,
}

impl BuildScope {
    /// A scope detached from any registry, whose boundaries are never marked.
    pub fn detached() -> Self {
        Self {
            state: Weak::new(),
            boundary: None,
            depth: 0,
        }
    }

    /// The outermost scope, under which the root boundary is registered at depth 0.
    pub(crate) fn new_root(state: &Rc<RefCell<BuildState>>) -> Self {
        Self {
            state: Rc::downgrade(state),
            boundary: None,
            depth: 0,
        }
    }

    /// The boundary the current subtree is reconciled under, if any.
    pub fn boundary(&self) -> Option<BuildBoundaryId> {
        self.boundary
    }

    /// Marks the element at `path` for a dependency-change rebuild on the next flush. A detached scope
    /// reaches no registry, so the mark is dropped.
    pub(crate) fn mark_dependency_changed(&self, target: &RoutingTarget) {
        if let Some(state) = self.state.upgrade() {
            state.borrow_mut().mark_dependency_changed(target);
        }
    }
}

/// A build boundary's persistent state, referenced by the registry while it is mounted.
struct BuildBoundaryCell {
    state: Weak<RefCell<BuildState>>,

    /// This boundary's key in the registry, replayed by the marks it queues.
    id: Cell<BuildBoundaryId>,

    /// The depth in the boundary nesting, sorted at flush so the owner re-enters boundaries rootmost-first.
    depth: usize,

    /// The inner element this boundary wraps.
    child: RefCell<Box<dyn AnyElement>>,

    /// The root of this boundary's render subtree, shared with the pipeline, so a rebuild can reconcile
    /// the render objects from here rather than walking the render tree from its root. Absent until the
    /// pipeline that owns the subtree attaches it.
    render: RefCell<Option<SharedRenderObject>>,

    /// The descendants marked since the last flush: each encoded path relative to the inner element,
    /// tagged with why it was marked.
    suffixes: RefCell<Vec<(Box<[u8]>, MarkKind)>>,

    /// Whether this boundary is currently in the registry's dirty list, guarding a double-mark from
    /// queueing it twice.
    is_dirty: Cell<bool>,

    /// The provided-value scope captured when this boundary was reconciled, so a targeted flush
    /// re-enters the subtree with the values an ancestor put in scope above it.
    provide: RefCell<ProvideScope>,
}

impl BuildBoundaryCell {
    fn register(ctx: &mut UpdateCtx) -> Rc<Self> {
        Self::register_under(ctx.build_scope(), ctx.provide_scope())
    }

    /// Registers a boundary under `scope`, capturing `provide`, with a placeholder child to be replaced
    /// once the inner element is built under [`child_scope`](Self::child_scope).
    fn register_under(scope: &BuildScope, provide: &ProvideScope) -> Rc<Self> {
        let depth = scope.depth;

        let cell = Rc::new(Self {
            state: Weak::clone(&scope.state),
            id: Cell::new(BuildBoundaryId::default()),
            depth,
            child: RefCell::new(Box::new(())),
            render: RefCell::new(None),
            suffixes: RefCell::new(Vec::new()),
            is_dirty: Cell::new(false),
            provide: RefCell::new(provide.clone()),
        });

        if let Some(state) = scope.state.upgrade() {
            cell.id
                .set(state.borrow_mut().boundaries.insert(Rc::downgrade(&cell)));
        }

        cell
    }

    /// The scope this boundary's children are reconciled under.
    fn child_scope(&self) -> BuildScope {
        BuildScope {
            state: Weak::clone(&self.state),
            boundary: Some(self.id.get()),
            depth: self.depth + 1,
        }
    }
}

impl Drop for BuildBoundaryCell {
    fn drop(&mut self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };

        let mut state = state.borrow_mut();
        let id = self.id.get();
        state.boundaries.remove(id);
        state.dirty.retain(|&dirty| dirty != id);
    }
}

impl BuildBoundaryCell {
    fn dispatch_within(&self, within: &RoutingPath, action: Dispatch) {
        let render = self.render.borrow();
        let Some(render) = render.as_ref() else {
            return;
        };
        let mut render = render.borrow_mut();

        let mut child = self.child.borrow_mut();
        child.dyn_dispatch(&mut *render, within, action);
    }

    fn clear_dirty(&self) {
        self.is_dirty.set(false);
    }

    fn flush_rebuilds(
        &self,
        scheduler: &mut dyn TaskScheduler,
        paint: &mut PaintPipeline,
        layout: &LayoutPipeline,
        scope: &PaintScope,
    ) {
        let suffixes = std::mem::take(&mut *self.suffixes.borrow_mut());
        let child_scope = self.child_scope();
        let provide = self.provide.borrow().clone();

        for (suffix, kind) in suffixes {
            // Pre-seed the routing path with the target's path under this boundary, so a task spawned during
            // the rebuild captures its own location rather than the boundary's.
            let mut path = suffix.to_vec();
            let mut ctx = UpdateCtx::new(
                scheduler,
                &mut path,
                &provide,
                &child_scope,
                layout,
                paint,
                scope,
            );

            let render = self.render.borrow();
            let Some(render) = render.as_ref() else {
                continue;
            };
            let mut render = render.borrow_mut();

            let action = match kind {
                MarkKind::Rebuild => Dispatch::Rebuild(&mut ctx),
                MarkKind::DependencyChanged => Dispatch::DependencyChanged(&mut ctx),
            };

            let mut child = self.child.borrow_mut();
            child.dyn_dispatch(&mut *render, RoutingPath::new(&suffix), action);
        }
    }
}

/// The root build boundary: a handle over the boundary's persistent state. Built by consuming a widget,
/// it produces the widget's render object once and registers the inner element so rebuilds reach it.
pub struct BuildBoundaryElement {
    cell: Rc<BuildBoundaryCell>,
}

impl BuildBoundaryElement {
    /// Registers an empty boundary with a placeholder inner element, for a pipeline whose root render
    /// object was built without a widget. It registers and dispatches like any other, but reaching its
    /// placeholder child does nothing.
    pub fn empty(scope: &BuildScope) -> Self {
        Self {
            cell: BuildBoundaryCell::register_under(scope, &ProvideScope::new()),
        }
    }

    /// Registers a boundary and builds `widget`'s element and render object under it, consuming the
    /// widget. The render object is held in a shared cell, returned to the caller and retained on the
    /// boundary, so a rebuild reconciles it from here while the pipeline lays out the same cell.
    pub fn create<V>(widget: V, ctx: &mut UpdateCtx) -> (Self, Rc<RefCell<V::Render>>)
    where
        V: Widget,
        V::Element: 'static,
        V::Render: RenderObject,
    {
        let cell = BuildBoundaryCell::register(ctx);

        let (element, render_object) =
            ctx.with_build_scope(&cell.child_scope(), |ctx| widget.create(ctx));

        *cell.child.borrow_mut() = Box::new(element);

        let render = Rc::new(RefCell::new(render_object));

        // Retain a protocol-agnostic view of the same cell. The unsize coercion needs a plain
        // binding: coercing at the `Rc::clone` call would unify it on the target type and refuse.
        let shared = Rc::clone(&render);
        let shared: SharedRenderObject = shared;
        *cell.render.borrow_mut() = Some(shared);

        (Self { cell }, render)
    }

    /// Reconciles the inner element and `render_object` against a new `widget` of the root's type.
    ///
    /// # Panics
    ///
    /// Panics if `widget`'s element type differs from the one the boundary was built with.
    pub fn update<V>(&mut self, widget: V, render_object: &mut V::Render, ctx: &mut UpdateCtx)
    where
        V: Widget,
        V::Element: 'static,
    {
        // An ancestor reconciling through this boundary may carry updated provides; recapture them so
        // a later targeted flush re-enters with the current scope, not the one seen at registration.
        *self.cell.provide.borrow_mut() = ctx.provide_scope().clone();

        let child_scope = self.cell.child_scope();

        ctx.with_build_scope(&child_scope, |ctx| {
            let mut child = self.cell.child.borrow_mut();
            let element = (**child)
                .as_any_mut()
                .downcast_mut::<V::Element>()
                .expect("root element does not match its widget's type");

            widget.update(element, render_object, ctx);
        });
    }

    /// The id of this boundary in its registry.
    pub fn id(&self) -> BuildBoundaryId {
        self.cell.id.get()
    }

    /// Captures the inner element's subtree as a diagnostics snapshot, into `d`.
    pub(crate) fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.cell.child.borrow().dyn_describe(d)
    }

    /// Routes `action` along `within` to the inner element. The path is relative to this boundary.
    pub fn dispatch(&mut self, within: &RoutingPath, action: Dispatch) {
        self.cell.dispatch_within(within, action);
    }
}

/// A widget whose subtree is a build boundary: a rebuild requested inside it reconciles only that
/// subtree, reached directly by its boundary id rather than by a walk from the root. Wrap a subtree
/// that rebuilds on its own so an unrelated rebuild elsewhere leaves it untouched.
pub struct RebuildBoundary<Child> {
    child: Child,
}

impl RebuildBoundary<()> {
    pub fn new() -> Self {
        Self { child: () }
    }

    pub fn child<Child>(self, child: Child) -> RebuildBoundary<Child> {
        RebuildBoundary { child }
    }
}

impl Default for RebuildBoundary<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Child> Widget for RebuildBoundary<Child>
where
    Child: Widget,
    Child::Element: 'static,
    Child::Render: RenderObject,
{
    type Element = RebuildBoundaryElement<Rc<RefCell<Child::Render>>>;

    type Render = Rc<RefCell<Child::Render>>;

    fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
        let (boundary, content) = BuildBoundaryElement::create(self.child, ctx);

        (
            RebuildBoundaryElement {
                boundary,
                _render: PhantomData,
            },
            content,
        )
    }

    fn update(
        self,
        element: &mut Self::Element,
        render_object: &mut Self::Render,
        ctx: &mut UpdateCtx,
    ) {
        // The parent-threaded render is the same shared object the boundary holds; reconcile the
        // inner subtree against it in place.
        let mut content = render_object.borrow_mut();
        element.boundary.update(self.child, &mut content, ctx);
    }
}

/// The [`Element`] of a [`RebuildBoundary`]. It names the shared render threaded to the parent while
/// the boundary it wraps holds the same render for an in-place rebuild.
pub struct RebuildBoundaryElement<R: ?Sized> {
    boundary: BuildBoundaryElement,
    _render: PhantomData<fn() -> R>,
}

impl<R> Element for RebuildBoundaryElement<R>
where
    R: ?Sized,
{
    type Render = R;

    fn dispatch(&mut self, _render: &mut R, path: &RoutingPath, action: Dispatch) {
        // The boundary holds its own render, so a dispatch routes through it rather than through the
        // parent-threaded render.
        self.boundary.dispatch(path, action);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.boundary.describe(d)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use crate::{
        context::{MessageCtx, UpdateCtx},
        element::{RoutingId, RoutingTarget},
        pipeline::PipelineOwner,
        scheduling::TaskHandle,
        test_fixtures::{Leaf, MultiChild},
        test_harness::TestCtx,
    };

    use super::{BuildBoundaryId, RebuildBoundary};

    /// A leaf that records the boundary it mounts under and counts the rebuilds delivered to it.
    fn probe(boundary: &Rc<Cell<Option<BuildBoundaryId>>>, rebuilds: &Rc<Cell<usize>>) -> Leaf {
        let boundary = Rc::clone(boundary);
        let rebuilds = Rc::clone(rebuilds);

        Leaf::new()
            .on_mount(move |ctx: &mut UpdateCtx| boundary.set(ctx.build_scope().boundary()))
            .on_message(MessageCtx::request_rebuild)
            .on_rebuild(move |_: &mut UpdateCtx| rebuilds.set(rebuilds.get() + 1))
    }

    #[test]
    fn each_rebuild_boundary_registers_its_own_boundary() {
        let a = Rc::new(Cell::new(None));
        let b = Rc::new(Cell::new(None));

        let widget = MultiChild {
            children: vec![
                RebuildBoundary::new().child(probe(&a, &Rc::new(Cell::new(0)))),
                RebuildBoundary::new().child(probe(&b, &Rc::new(Cell::new(0)))),
            ],
        };

        let mut tasks = TestCtx::new();
        let owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        let a = a.get().expect("the first leaf mounted under a boundary");
        let b = b.get().expect("the second leaf mounted under a boundary");

        assert_ne!(
            a,
            owner.root_id(),
            "a RebuildBoundary is its own boundary, not the root"
        );
        assert_ne!(a, b, "sibling boundaries are distinct");
    }

    #[test]
    fn marking_a_rebuild_boundary_rebuilds_only_its_subtree() {
        let a_boundary = Rc::new(Cell::new(None));
        let a_rebuilds = Rc::new(Cell::new(0));
        let b_boundary = Rc::new(Cell::new(None));
        let b_rebuilds = Rc::new(Cell::new(0));

        let widget = MultiChild {
            children: vec![
                RebuildBoundary::new().child(probe(&a_boundary, &a_rebuilds)),
                RebuildBoundary::new().child(probe(&b_boundary, &b_rebuilds)),
            ],
        };

        let mut tasks = TestCtx::new();
        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        let a = a_boundary.get().unwrap();
        let b = b_boundary.get().unwrap();

        // Each leaf sits at the empty path within its own boundary, reached by that boundary's id; a
        // message there asks it to rebuild, the way a set-state would.
        owner.dispatch_message(&RoutingTarget::new(a, Vec::new()), Box::new(()));
        assert!(owner.flush_build(&mut tasks.scheduler()));
        assert_eq!(a_rebuilds.get(), 1);
        assert_eq!(b_rebuilds.get(), 0, "the sibling boundary was not rebuilt");

        owner.dispatch_message(&RoutingTarget::new(b, Vec::new()), Box::new(()));
        assert!(owner.flush_build(&mut tasks.scheduler()));
        assert_eq!(
            a_rebuilds.get(),
            1,
            "the first boundary was not rebuilt again"
        );
        assert_eq!(b_rebuilds.get(), 1);
    }

    /// A shared counter, so a widget's closures can be `'static` and still be observed by the test.
    fn counter() -> Rc<Cell<usize>> {
        Rc::new(Cell::new(0))
    }

    /// A rebuild handler that bumps `c`.
    fn bump(c: &Rc<Cell<usize>>) -> impl Fn(&mut UpdateCtx) + 'static {
        let c = Rc::clone(c);
        move |_| c.set(c.get() + 1)
    }

    #[test]
    fn message_dirties_target_and_rebuild_only_reaches_that_target() {
        use crate::test_fixtures::Transparent;

        //   MultiChild
        //   ├─ [0] Transparent -> Leaf (no rebuild)
        //   ├─ [1] Transparent -> Leaf (requests rebuild)
        //   └─ [2] Transparent -> Leaf (no rebuild)
        let r0 = counter();
        let r1 = counter();
        let r2 = counter();

        let widget = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new().on_rebuild(bump(&r0)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r1)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(bump(&r2)),
                },
            ],
        };

        let mut tasks = TestCtx::new();
        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        let target =
            RoutingTarget::new(owner.root_id(), RoutingId::encode_path([RoutingId::new(1)]));
        owner.dispatch_message(&target, Box::new(42_u32));
        assert!(
            owner.is_dirty(),
            "the addressed element requested a rebuild"
        );

        assert!(owner.flush_build(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 0);
        assert_eq!(r1.get(), 1);
        assert_eq!(r2.get(), 0);
    }

    #[test]
    fn rebuild_reaches_every_dirtied_target() {
        use crate::test_fixtures::Transparent;

        let r0 = counter();
        let r1 = counter();
        let r2 = counter();

        let widget = MultiChild {
            children: vec![
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r0)),
                },
                Transparent {
                    child: Leaf::new().on_rebuild(bump(&r1)),
                },
                Transparent {
                    child: Leaf::new()
                        .on_message(MessageCtx::request_rebuild)
                        .on_rebuild(bump(&r2)),
                },
            ],
        };

        let mut tasks = TestCtx::new();
        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        let root = owner.root_id();
        owner.dispatch_message(
            &RoutingTarget::new(root, RoutingId::encode_path([RoutingId::new(0)])),
            Box::new(1_u32),
        );
        owner.dispatch_message(
            &RoutingTarget::new(root, RoutingId::encode_path([RoutingId::new(2)])),
            Box::new(2_u32),
        );

        assert!(owner.flush_build(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 1);
        assert_eq!(r1.get(), 0);
        assert_eq!(r2.get(), 1);
    }

    #[test]
    fn flush_with_empty_set_is_noop() {
        let r0 = counter();

        let widget = Leaf::new().on_rebuild(bump(&r0));

        let mut tasks = TestCtx::new();
        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        assert!(!owner.is_dirty());
        assert!(!owner.flush_build(&mut tasks.scheduler()));

        assert_eq!(r0.get(), 0);
    }

    #[test]
    fn spawned_task_posts_message_back_to_its_element() {
        // A leaf spawns a task on mount and stashes its handle (as a real element would) so the
        // task outlives the build. The runner drives it to completion; the task posts a message
        // back to its own routing path, which dispatching then delivers to the same leaf.
        let mut tasks = TestCtx::new();

        let received = Rc::new(Cell::new(None::<u32>));
        let handle = Rc::new(RefCell::new(None::<TaskHandle>));

        let widget = Leaf::new()
            .on_mount({
                let handle = Rc::clone(&handle);
                move |ctx| {
                    *handle.borrow_mut() = Some(
                        ctx.spawn(|task| async move {
                            task.send(7_u32);
                        })
                        .expect("scheduler available during build"),
                    );
                }
            })
            .on_message({
                let received = Rc::clone(&received);
                move |ctx| received.set(Some(ctx.consume::<u32>()))
            });

        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        tasks.run_tasks_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(&path, message);
        }

        assert_eq!(received.get(), Some(7));
    }

    #[test]
    fn task_message_rebuilds_only_the_messaged_child() {
        let mut tasks = TestCtx::new();

        let a_messages = counter();
        let a_rebuilds = counter();
        let b_rebuilds = counter();
        let a_handle = Rc::new(RefCell::new(None::<TaskHandle>));

        let widget = MultiChild {
            children: vec![
                Leaf::new()
                    .on_mount({
                        let a_handle = Rc::clone(&a_handle);
                        move |ctx| {
                            *a_handle.borrow_mut() = Some(
                                ctx.spawn(|task| async move {
                                    task.send(42_u32);
                                })
                                .expect("scheduler available during build"),
                            );
                        }
                    })
                    .on_message({
                        let a_messages = Rc::clone(&a_messages);
                        move |ctx| {
                            a_messages.set(a_messages.get() + 1);
                            let _ = ctx.consume::<u32>();
                            ctx.request_rebuild();
                        }
                    })
                    .on_rebuild(bump(&a_rebuilds)),
                Leaf::new().on_rebuild(bump(&b_rebuilds)),
            ],
        };

        let mut owner = PipelineOwner::new(widget, &mut tasks.scheduler());

        tasks.run_tasks_to_completion();

        let messages: Vec<_> = tasks.messages().collect();
        assert_eq!(messages.len(), 1, "the task posted exactly one message");

        for (path, message) in messages {
            owner.dispatch_message(&path, message);
        }

        assert_eq!(a_messages.get(), 1);
        assert!(owner.is_dirty(), "only the messaged child was dirtied");

        assert!(owner.flush_build(&mut tasks.scheduler()));

        assert_eq!(a_rebuilds.get(), 1, "child 0 rebuilt");
        assert_eq!(b_rebuilds.get(), 0, "the sibling was not rebuilt");
    }
}
