use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use slotmap::SlotMap;

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::{AnyElement, RoutingId, RoutingPath},
    provide::ProvideScope,
    render_object::{AnyRenderObject, RenderObject},
    scheduling::TaskScheduler,
    widget::Widget,
};

type SharedRenderObject = Rc<RefCell<dyn AnyRenderObject>>;

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
}

/// Delivers `ctx` to the element at `path`, marking its boundary for rebuild if the element asks. A path
/// whose boundary is gone (unmounted, or its inner replaced) is dropped.
pub fn deliver_message(state: &Rc<RefCell<BuildState>>, path: &RoutingPath, ctx: &mut MessageCtx) {
    let Some(cell) = state.borrow().lookup(path.boundary()) else {
        return;
    };

    cell.dispatch_within(path.within(), Dispatch::Message(ctx));

    if ctx.rebuild_requested() {
        cell.mark(path.within());
    }
}

/// Marks the element at `path` to rebuild on the next flush. A path whose boundary is gone is dropped.
pub fn mark_rebuild(state: &Rc<RefCell<BuildState>>, path: &RoutingPath) {
    if let Some(cell) = state.borrow().lookup(path.boundary()) {
        cell.mark(path.within());
    }
}

/// Rebuilds every boundary marked since the last flush. Returns whether anything rebuilt.
pub fn flush_boundaries(
    state: &Rc<RefCell<BuildState>>,
    scheduler: &mut dyn TaskScheduler,
    provide: &ProvideScope,
) -> bool {
    let ordered = state.borrow_mut().drain_rootmost_first();

    if ordered.is_empty() {
        return false;
    }

    for cell in ordered {
        cell.flush_rebuilds(scheduler, provide);
    }

    true
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

    /// The paths, relative to the inner element, of the descendants marked for rebuild.
    suffixes: RefCell<Vec<Box<[RoutingId]>>>,

    /// Whether this boundary is currently in the registry's dirty list, guarding a double-mark from
    /// queueing it twice.
    is_dirty: Cell<bool>,
}

impl BuildBoundaryCell {
    /// Registers a boundary under the current scope, with a placeholder child to be replaced once the
    /// inner element is built under [`child_scope`](Self::child_scope).
    fn register(ctx: &mut UpdateCtx) -> Rc<Self> {
        let scope = ctx.build_scope().clone();
        let depth = scope.depth;

        let cell = Rc::new(Self {
            state: Weak::clone(&scope.state),
            id: Cell::new(BuildBoundaryId::default()),
            depth,
            child: RefCell::new(Box::new(())),
            render: RefCell::new(None),
            suffixes: RefCell::new(Vec::new()),
            is_dirty: Cell::new(false),
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

    /// Marks the descendant at `within` for rebuild, registering this boundary on the clean-to-dirty edge.
    fn mark_within(&self, within: &[RoutingId]) {
        let Some(state) = self.state.upgrade() else {
            return;
        };

        if !self.is_dirty.get() {
            self.is_dirty.set(true);
            state.borrow_mut().dirty.push(self.id.get());
        }

        self.suffixes.borrow_mut().push(within.into());
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
    fn dispatch_within(&self, within: &[RoutingId], action: Dispatch) {
        let render = self.render.borrow();
        let Some(render) = render.as_ref() else {
            return;
        };
        let mut render = render.borrow_mut();

        let mut child = self.child.borrow_mut();
        child.dyn_dispatch(&mut *render, within, action);
    }

    fn mark(&self, within: &[RoutingId]) {
        self.mark_within(within);
    }

    fn clear_dirty(&self) {
        self.is_dirty.set(false);
    }

    fn flush_rebuilds(&self, scheduler: &mut dyn TaskScheduler, provide: &ProvideScope) {
        let suffixes = std::mem::take(&mut *self.suffixes.borrow_mut());
        let child_scope = self.child_scope();

        for suffix in suffixes {
            // Pre-seed the routing path with the target's path under this boundary, so a task spawned during
            // the rebuild captures its own location rather than the boundary's.
            let mut path = suffix.to_vec();
            let mut ctx = UpdateCtx::new(scheduler, &mut path, provide, &child_scope);

            let render = self.render.borrow();
            let Some(render) = render.as_ref() else {
                continue;
            };
            let mut render = render.borrow_mut();

            let mut child = self.child.borrow_mut();
            child.dyn_dispatch(&mut *render, &suffix, Dispatch::Rebuild(&mut ctx));
        }
    }
}

/// The root build boundary: a handle over the boundary's persistent state. Built by consuming a widget,
/// it produces the widget's render object once and registers the inner element so rebuilds reach it.
pub struct BuildBoundaryElement {
    cell: Rc<BuildBoundaryCell>,
}

impl BuildBoundaryElement {
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

    /// Routes `action` along `within` to the inner element. The path is relative to this boundary.
    pub fn dispatch(&mut self, within: &[RoutingId], action: Dispatch) {
        self.cell.dispatch_within(within, action);
    }
}
