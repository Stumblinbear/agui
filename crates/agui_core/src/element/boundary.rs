use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::Arc,
};

use slotmap::SlotMap;

use crate::{
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::{AnyElement, Element, RoutingId, RoutingPath},
    provide::ProvideScope,
    scheduling::TaskScheduler,
    widget::AnyWidget,
};

slotmap::new_key_type! {
    /// Identifies a registered build boundary within one [`BuildState`].
    pub struct BoundaryId;
}

/// The registry of build boundaries, shared between the owner that flushes them, the boundaries that mark
/// themselves, and the routing paths that address them.
///
/// Every element lives under exactly one boundary; the root is the outermost, registered like any other.
/// A boundary is addressed by its [`BoundaryId`], so a message or rebuild reaches it without walking from
/// the root.
pub struct BuildState {
    boundaries: SlotMap<BoundaryId, Weak<dyn DynBoundary>>,
    dirty: Vec<BoundaryId>,
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

    fn lookup(&self, id: BoundaryId) -> Option<Rc<dyn DynBoundary>> {
        self.boundaries.get(id).and_then(Weak::upgrade)
    }

    /// Drains the marked boundaries into their owning `Rc`s, ordered shallowest-depth first so that
    /// re-dispatching an outer boundary, which reconciles the boundaries nested in it, lets the inner ones
    /// be skipped rather than rebuilt twice.
    fn drain_rootmost_first(&mut self) -> Vec<Rc<dyn DynBoundary>> {
        let ids: Vec<BoundaryId> = self.dirty.drain(..).collect();

        let mut ordered: Vec<Rc<dyn DynBoundary>> = ids
            .into_iter()
            .filter_map(|id| self.boundaries.get(id).and_then(Weak::upgrade))
            .collect();

        for cell in &ordered {
            cell.clear_dirty();
        }

        ordered.sort_by_key(|cell| cell.depth());

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
    boundary: Option<BoundaryId>,
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
    pub fn boundary(&self) -> Option<BoundaryId> {
        self.boundary
    }
}

/// A build boundary's persistent state, owned by its [`BuildBoundaryElement`] handle and referenced by the
/// registry while it is mounted. It retains the inner widget so a rebuild dispatches into the inner element
/// directly, without the root holding its recipe.
struct BoundaryCell<R> {
    state: Weak<RefCell<BuildState>>,

    /// This boundary's key in the registry, replayed by the marks it queues. Replaced when the inner widget
    /// is swapped, so a path addressing the old inner is dropped at lookup.
    id: Cell<BoundaryId>,

    /// The depth in the boundary nesting, sorted at flush so the owner re-enters boundaries rootmost-first.
    depth: usize,

    /// The inner widget, retained so a rebuild reaches it without the root holding its recipe.
    recipe: RefCell<RetainedRecipe<R>>,

    /// The inner element this boundary wraps.
    child: RefCell<Box<dyn AnyElement>>,

    /// The paths, relative to the inner element, of the descendants marked for rebuild.
    suffixes: RefCell<Vec<Box<[RoutingId]>>>,

    /// Whether this boundary is currently in the registry's dirty list, guarding a double-mark from
    /// queueing it twice.
    is_dirty: Cell<bool>,
}

impl<R: 'static> BoundaryCell<R> {
    fn create(recipe: RetainedRecipe<R>, ctx: &mut UpdateCtx) -> Rc<Self> {
        let scope = ctx.build_scope().clone();
        let depth = scope.depth;

        let cell = Rc::new(Self {
            state: Weak::clone(&scope.state),
            id: Cell::new(BoundaryId::default()),
            depth,
            recipe: RefCell::new(recipe),
            child: RefCell::new(Box::new(())),
            suffixes: RefCell::new(Vec::new()),
            is_dirty: Cell::new(false),
        });

        if let Some(state) = scope.state.upgrade() {
            let typed = Rc::downgrade(&cell);
            let weak: Weak<dyn DynBoundary> = typed;
            cell.id.set(state.borrow_mut().boundaries.insert(weak));
        }

        let child = ctx.with_build_scope(&cell.child_scope(), |ctx| {
            cell.recipe.borrow().get().dyn_create_element(ctx)
        });
        *cell.child.borrow_mut() = child;

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

impl<R> Drop for BoundaryCell<R> {
    fn drop(&mut self) {
        let Some(state) = self.state.upgrade() else {
            return;
        };

        let mut state = state.borrow_mut();
        state.boundaries.remove(self.id.get());
        let id = self.id.get();
        state.dirty.retain(|&dirty| dirty != id);
    }
}

/// The behavior of a [`BoundaryCell`], erased so the registry can hold boundaries of differing render
/// types in one map.
trait DynBoundary {
    fn dispatch_within(&self, within: &[RoutingId], action: Dispatch);

    fn mark(&self, within: &[RoutingId]);

    fn depth(&self) -> usize;

    fn clear_dirty(&self);

    fn flush_rebuilds(&self, scheduler: &mut dyn TaskScheduler, provide: &ProvideScope);
}

impl<R: 'static> DynBoundary for BoundaryCell<R> {
    fn dispatch_within(&self, within: &[RoutingId], action: Dispatch) {
        self.recipe
            .borrow()
            .get()
            .dyn_dispatch(&mut **self.child.borrow_mut(), within, action);
    }

    fn mark(&self, within: &[RoutingId]) {
        self.mark_within(within);
    }

    fn depth(&self) -> usize {
        self.depth
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

            self.recipe.borrow().get().dyn_dispatch(
                &mut **self.child.borrow_mut(),
                &suffix,
                Dispatch::Rebuild(&mut ctx),
            );
        }
    }
}

/// The retained inner widget of a boundary, in whichever shared pointer wrapped it.
enum RetainedRecipe<R> {
    Rc(Rc<dyn AnyWidget<Render = R>>),
    Arc(Arc<dyn AnyWidget<Render = R>>),
}

impl<R> RetainedRecipe<R> {
    fn get(&self) -> &dyn AnyWidget<Render = R> {
        match self {
            Self::Rc(recipe) => &**recipe,
            Self::Arc(recipe) => &**recipe,
        }
    }
}

/// The [`Element`] of a shared-pointer build boundary. A thin handle over the boundary's persistent state,
/// so the element can move with reconcile while the boundary itself stays put and registered.
pub struct BuildBoundaryElement<R: 'static> {
    cell: Rc<BoundaryCell<R>>,
}

impl<R: 'static> Element for BuildBoundaryElement<R> {}

impl<R: 'static> BuildBoundaryElement<R> {
    /// Creates a boundary whose inner widget is retained in an [`Rc`].
    pub fn create_rc(recipe: Rc<dyn AnyWidget<Render = R>>, ctx: &mut UpdateCtx) -> Self {
        Self {
            cell: BoundaryCell::create(RetainedRecipe::Rc(recipe), ctx),
        }
    }

    /// Creates a boundary whose inner widget is retained in an [`Arc`].
    pub fn create_arc(recipe: Arc<dyn AnyWidget<Render = R>>, ctx: &mut UpdateCtx) -> Self {
        Self {
            cell: BoundaryCell::create(RetainedRecipe::Arc(recipe), ctx),
        }
    }

    /// Reconciles against an [`Rc`]-retained `new`.
    pub fn update_rc(&mut self, recipe: Rc<dyn AnyWidget<Render = R>>, ctx: &mut UpdateCtx) {
        self.update(RetainedRecipe::Rc(recipe), ctx);
    }

    /// Reconciles against an [`Arc`]-retained `new`.
    pub fn update_arc(&mut self, recipe: Arc<dyn AnyWidget<Render = R>>, ctx: &mut UpdateCtx) {
        self.update(RetainedRecipe::Arc(recipe), ctx);
    }

    /// The id of this boundary in its registry.
    pub fn id(&self) -> BoundaryId {
        self.cell.id.get()
    }

    /// Reconciles the inner element in place when its type is unchanged; otherwise replaces the whole
    /// boundary, so the stale id drops any rebuilds still pending for the old inner.
    fn update(&mut self, recipe: RetainedRecipe<R>, ctx: &mut UpdateCtx) {
        let same = self
            .cell
            .recipe
            .borrow()
            .get()
            .dyn_is_same_type(recipe.get());

        if !same {
            self.cell = BoundaryCell::create(recipe, ctx);
            return;
        }

        let old = std::mem::replace(&mut *self.cell.recipe.borrow_mut(), recipe);
        let child_scope = self.cell.child_scope();

        ctx.with_build_scope(&child_scope, |ctx| {
            self.cell.recipe.borrow().get().dyn_update(
                &mut self.cell.child.borrow_mut(),
                old.get(),
                ctx,
            );
        });
    }

    /// Routes `action` along `within` to the inner element. The path is relative to this boundary.
    pub fn dispatch(&mut self, within: &[RoutingId], action: Dispatch) {
        self.cell.dispatch_within(within, action);
    }

    /// Produces the render object from the retained inner widget and its element.
    pub fn create_render_object(&self) -> R {
        self.cell
            .recipe
            .borrow()
            .get()
            .dyn_create_render_object(&**self.cell.child.borrow())
    }

    /// Syncs `render_object` from the retained inner widget and its element.
    pub fn update_render_object(&self, render_object: &mut R) {
        self.cell
            .recipe
            .borrow()
            .get()
            .dyn_update_render_object(&**self.cell.child.borrow(), render_object);
    }
}
