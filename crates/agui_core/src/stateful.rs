use crate::{
    context::Dispatch,
    diagnostics::{Diagnostics, DiagnosticsNode, DiagnosticsNodeBuilder},
    element::{Element, RoutingId, node::ElementNode},
    prelude::element::UpdateCtx,
    widget::Widget,
};

/// State that persists across rebuilds and produces the subtree to show for it.
///
/// Implement this on the data a widget owns; [`build`](Self::build) reads that data and returns the
/// child to display.
pub trait WidgetState {
    type Widget: Widget;

    type Child: Widget;

    fn init_state(ctx: &mut UpdateCtx, widget: Self::Widget) -> Self;

    fn did_update_widget(&mut self, ctx: &mut UpdateCtx, widget: Self::Widget);

    /// Reacts to a change in a value this state depends on, before the rebuild's
    /// [`build`](Self::build). Recompute derived state or re-establish anything keyed by the
    /// dependency here. The default does nothing.
    fn did_change_dependencies(&mut self, ctx: &mut UpdateCtx) {
        let _ = ctx;
    }

    /// Builds the subtree to show for the current state.
    fn build(&self, ctx: &mut UpdateCtx) -> Self::Child;

    /// Adds this state's data to `node`, for a diagnostics dump.
    fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
        node
    }
}

/// A mutation applied to a [`State`] to change it, delivered as a message to a [`Stateful`] widget.
///
/// Deliver one to the widget's path to mutate its state and schedule a rebuild of its subtree.
pub type SetState<S> = Box<dyn FnOnce(&mut S)>;

/// The [`Element`] of a [`Stateful`] widget. It owns the state and the child's materialized subtree.
pub struct StatefulElement<S>
where
    S: WidgetState,
{
    state: S,

    child: ElementNode<<S::Child as Widget>::Element>,
}

impl<S> StatefulElement<S>
where
    S: WidgetState,
{
    pub fn new(state: S, child: <S::Child as Widget>::Element) -> Self {
        Self {
            state,

            child: ElementNode::new(child),
        }
    }
}

impl<S> Element for StatefulElement<S>
where
    S: WidgetState + 'static,
    S::Child: Widget,
{
    type Render = <S::Child as Widget>::Render;

    fn dispatch(&mut self, render: &mut Self::Render, path: &[RoutingId], action: Dispatch) {
        let Some((_, rest)) = path.split_first() else {
            match action {
                Dispatch::Message(ctx) => {
                    let apply: SetState<S> = ctx.consume();
                    apply(&mut self.state);
                    ctx.request_rebuild();
                }

                Dispatch::Rebuild(ctx) => {
                    let child = self.state.build(ctx);
                    child.update(&mut self.child.element, render, ctx);
                }

                Dispatch::DependencyChanged(ctx) => {
                    self.state.did_change_dependencies(ctx);
                    let child = self.state.build(ctx);
                    child.update(&mut self.child.element, render, ctx);
                }
            }

            return;
        };

        self.child.element.dispatch(render, rest, action);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.state
            .describe(d.node_for::<S::Widget>())
            .child(|d| self.child.element.describe(d))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use typed_floats::{Positive, PositiveFinite};

    use super::*;
    use crate::{
        context::{Dispatch, LayoutCtx, MountCtx, PaintCtx},
        diagnostics::{Diagnostics, DiagnosticsNodeBuilder},
        element::{Element, LeafElement, RoutingPath},
        geometry::{Offset, Size},
        input::hit_test::{HitTest, HitTestResult},
        pipeline::{PipelineOwner, layout::LayoutPipeline, paint::PaintPipeline},
        prelude::{element::UpdateCtx, render_object::LayoutScope},
        render_object::{
            RenderObject,
            box_layout::{BoxConstraints, RenderBox},
        },
        test_fixtures::MultiChild,
        test_harness::TestCtx,
        text::TextBaseline,
    };

    /// State holding a count, building a square sized to it. `creates` tallies how many child render
    /// objects were built, so a test can tell a reconcile (reused) from a replacement (rebuilt).
    struct Counter {
        count: u32,

        creates: Rc<Cell<usize>>,
    }

    impl Widget for Counter {
        type Element = StatefulElement<CounterState>;

        type Render = <<CounterState as WidgetState>::Child as Widget>::Render;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let state = CounterState::init_state(ctx, self);
            let child = state.build(ctx);
            let (child_element, child_render) = child.create(ctx);
            let element = StatefulElement::new(state, child_element);
            (element, child_render)
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.state.did_update_widget(ctx, self);
            let child = element.state.build(ctx);
            child.update(&mut element.child.element, render, ctx);
        }
    }

    struct CounterState {
        count: u32,

        creates: Rc<Cell<usize>>,
    }

    impl WidgetState for CounterState {
        type Widget = Counter;
        type Child = Square;

        fn init_state(_: &mut UpdateCtx, widget: Self::Widget) -> Self {
            CounterState {
                count: widget.count,
                creates: widget.creates,
            }
        }

        fn did_update_widget(&mut self, _: &mut UpdateCtx, widget: Self::Widget) {
            self.count = widget.count;
            self.creates = widget.creates;
        }

        #[allow(clippy::cast_precision_loss)]
        fn build(&self, _: &mut UpdateCtx) -> Self::Child {
            Square {
                side: self.count as f32,
                creates: Rc::clone(&self.creates),
            }
        }

        fn describe<'a>(&self, node: DiagnosticsNodeBuilder<'a>) -> DiagnosticsNodeBuilder<'a> {
            node.property("count", self.count)
        }
    }

    /// A leaf laying out to a square of `side`, whose render object records its own creation.
    struct Square {
        side: f32,
        creates: Rc<Cell<usize>>,
    }

    impl Widget for Square {
        type Element = LeafElement<RenderSquare>;

        type Render = RenderSquare;

        fn create(self, _: &mut UpdateCtx) -> (LeafElement<RenderSquare>, RenderSquare) {
            self.creates.set(self.creates.get() + 1);

            (LeafElement::new(), RenderSquare { side: self.side })
        }

        fn update(
            self,
            _: &mut LeafElement<RenderSquare>,
            render: &mut RenderSquare,
            _: &mut UpdateCtx,
        ) {
            render.side = self.side;
        }
    }

    struct RenderSquare {
        side: f32,
    }

    impl RenderObject for RenderSquare {
        fn mount(&mut self, _: &mut MountCtx) {}

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for RenderSquare {
        fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
            PositiveFinite::try_from(self.side).ok()
        }

        fn measure(&self, constraints: BoxConstraints) -> Size {
            constraints.constrain(Size::new(self.side, self.side))
        }

        fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
            constraints.constrain(Size::new(self.side, self.side))
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

    /// A stateful fixture that tallies its builds and its dependency-change hook calls, so a test
    /// can tell a dependency change from a plain rebuild.
    struct DepWidget {
        builds: Rc<Cell<usize>>,
        dep_changes: Rc<Cell<usize>>,
    }

    struct DepState {
        builds: Rc<Cell<usize>>,
        dep_changes: Rc<Cell<usize>>,
    }

    impl Widget for DepWidget {
        type Element = StatefulElement<DepState>;

        type Render = <<DepState as WidgetState>::Child as Widget>::Render;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let state = DepState::init_state(ctx, self);
            let child = state.build(ctx);
            let (child_element, child_render) = child.create(ctx);
            (StatefulElement::new(state, child_element), child_render)
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.state.did_update_widget(ctx, self);
            let child = element.state.build(ctx);
            child.update(&mut element.child.element, render, ctx);
        }
    }

    impl WidgetState for DepState {
        type Widget = DepWidget;
        type Child = Square;

        fn init_state(_: &mut UpdateCtx, widget: Self::Widget) -> Self {
            DepState {
                builds: widget.builds,
                dep_changes: widget.dep_changes,
            }
        }

        fn did_update_widget(&mut self, _: &mut UpdateCtx, widget: Self::Widget) {
            self.builds = widget.builds;
            self.dep_changes = widget.dep_changes;
        }

        fn did_change_dependencies(&mut self, _: &mut UpdateCtx) {
            self.dep_changes.set(self.dep_changes.get() + 1);
        }

        fn build(&self, _: &mut UpdateCtx) -> Self::Child {
            self.builds.set(self.builds.get() + 1);
            Square {
                side: 1.0,
                creates: Rc::new(Cell::new(0)),
            }
        }
    }

    #[test]
    fn dependency_change_runs_the_hook_then_rebuilds() {
        let builds = Rc::new(Cell::new(0));
        let deps = Rc::new(Cell::new(0));

        let (mut element, mut render) = TestCtx::new().create(DepWidget {
            builds: Rc::clone(&builds),
            dep_changes: Rc::clone(&deps),
        });

        let after_create = builds.get();
        assert_eq!(deps.get(), 0, "create does not run the dependency hook");

        TestCtx::new()
            .run(|ctx| element.dispatch(&mut render, &[], Dispatch::DependencyChanged(ctx)));
        assert_eq!(
            deps.get(),
            1,
            "a dependency change runs did_change_dependencies"
        );
        assert_eq!(builds.get(), after_create + 1, "and then rebuilds");

        TestCtx::new().run(|ctx| element.dispatch(&mut render, &[], Dispatch::Rebuild(ctx)));
        assert_eq!(
            deps.get(),
            1,
            "a plain rebuild does not run did_change_dependencies"
        );
        assert_eq!(builds.get(), after_create + 2, "but still rebuilds");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn set_state_reconciles_the_child_render_in_place() {
        let creates = Rc::new(Cell::new(0));

        let mut tasks = TestCtx::new();
        let mut owner = PipelineOwner::new(
            Counter {
                count: 1,

                creates: Rc::clone(&creates),
            },
            &mut tasks.scheduler(),
        );

        assert_eq!(creates.get(), 1);
        assert!(
            owner.diagnostics().to_string().contains("count=1"),
            "the state starts at one"
        );

        // A set-state delivered to the widget's own path mutates the state and asks for a rebuild.
        let bump: SetState<CounterState> = Box::new(|state| state.count += 1);
        owner.dispatch_message(
            &RoutingPath::new(owner.root_id(), Vec::new()),
            Box::new(bump),
        );
        assert!(owner.is_dirty(), "set-state requested a rebuild");

        assert!(owner.flush_build(&mut tasks.scheduler()));

        assert!(
            owner.diagnostics().to_string().contains("count=2"),
            "the rebuild reconciled the state to its new value"
        );
        assert_eq!(
            creates.get(),
            1,
            "the same-type child reused its render object rather than rebuilding it"
        );
    }

    #[test]
    fn the_wrapper_presents_its_child_unchanged() {
        let creates = Rc::new(Cell::new(0));

        let (_element, mut render) = TestCtx::new().create(Counter {
            count: 5,

            creates: Rc::clone(&creates),
        });

        let layout = LayoutPipeline::default();
        let mut paint = PaintPipeline::default();

        let size = render.layout(
            &mut LayoutCtx::new(&layout, &mut paint, LayoutScope::detached()),
            BoxConstraints::loose(Size::new(100, 100)),
        );

        assert_eq!(
            size,
            Size::new(5.0, 5.0),
            "the wrapper reports its child's size"
        );
    }

    #[test]
    fn describe_includes_state_properties() {
        let (element, _render) = TestCtx::new().create(Counter {
            count: 7,
            creates: Rc::new(Cell::new(0)),
        });

        let dump = element.describe(&mut Diagnostics::new()).to_string();

        assert!(dump.starts_with("Counter  count=7"), "dump was:\n{dump}");
    }

    /// A render object that tallies how many times it was mounted.
    struct RenderMountProbe {
        mounts: Rc<Cell<usize>>,
    }

    impl RenderObject for RenderMountProbe {
        fn mount(&mut self, _: &mut MountCtx) {
            self.mounts.set(self.mounts.get() + 1);
        }

        fn unmount(&mut self, _: &mut MountCtx) {}

        fn update_compositing_bits(&mut self) -> bool {
            false
        }
    }

    impl RenderBox for RenderMountProbe {
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

    /// A leaf whose render object tallies its mounts onto a shared counter.
    struct MountProbe {
        mounts: Rc<Cell<usize>>,
    }

    impl Widget for MountProbe {
        type Element = LeafElement<RenderMountProbe>;

        type Render = RenderMountProbe;

        fn create(self, _: &mut UpdateCtx) -> (LeafElement<RenderMountProbe>, RenderMountProbe) {
            (
                LeafElement::new(),
                RenderMountProbe {
                    mounts: self.mounts,
                },
            )
        }

        fn update(
            self,
            _: &mut LeafElement<RenderMountProbe>,
            render: &mut RenderMountProbe,
            _: &mut UpdateCtx,
        ) {
            render.mounts = self.mounts;
        }
    }

    /// A stateful widget that builds a list of `count` [`MountProbe`]s, so bumping the count appends a
    /// child and creates a fresh render object on the rebuild.
    struct Grower {
        count: u32,
        mounts: Rc<Cell<usize>>,
    }

    struct GrowerState {
        count: u32,
        mounts: Rc<Cell<usize>>,
    }

    impl Widget for Grower {
        type Element = StatefulElement<GrowerState>;

        type Render = <<GrowerState as WidgetState>::Child as Widget>::Render;

        fn create(self, ctx: &mut UpdateCtx) -> (Self::Element, Self::Render) {
            let state = GrowerState::init_state(ctx, self);
            let child = state.build(ctx);
            let (child_element, child_render) = child.create(ctx);
            (StatefulElement::new(state, child_element), child_render)
        }

        fn update(
            self,
            element: &mut Self::Element,
            render: &mut Self::Render,
            ctx: &mut UpdateCtx,
        ) {
            element.state.did_update_widget(ctx, self);
            let child = element.state.build(ctx);
            child.update(&mut element.child.element, render, ctx);
        }
    }

    impl WidgetState for GrowerState {
        type Widget = Grower;
        type Child = MultiChild<MountProbe>;

        fn init_state(_: &mut UpdateCtx, widget: Self::Widget) -> Self {
            GrowerState {
                count: widget.count,
                mounts: widget.mounts,
            }
        }

        fn did_update_widget(&mut self, _: &mut UpdateCtx, widget: Self::Widget) {
            self.count = widget.count;
            self.mounts = widget.mounts;
        }

        fn build(&self, _: &mut UpdateCtx) -> Self::Child {
            MultiChild {
                children: (0..self.count)
                    .map(|_| MountProbe {
                        mounts: Rc::clone(&self.mounts),
                    })
                    .collect(),
            }
        }
    }

    /// A render object created by a rebuild should be mounted, the same as one in the initial tree:
    /// otherwise it never captures its enclosing boundary and can never mark itself for paint.
    #[test]
    fn a_render_object_created_on_rebuild_is_mounted() {
        let mounts = Rc::new(Cell::new(0));

        let mut ctx = TestCtx::new();
        let (mut owner, view) = ctx.mount_view(Grower {
            count: 1,
            mounts: Rc::clone(&mounts),
        });

        view.resize(BoxConstraints::tight(Size::new(100, 100)));
        owner.flush_layout();
        owner.flush_paint();

        assert_eq!(
            mounts.get(),
            1,
            "the initial child is mounted with the root"
        );

        // Bump the count so the rebuild appends a second child, creating a new render object.
        let grow: SetState<GrowerState> = Box::new(|state| state.count = 2);
        owner.dispatch_message(
            &RoutingPath::new(owner.root_id(), Vec::new()),
            Box::new(grow),
        );
        assert!(owner.flush_build(&mut ctx.scheduler()));

        owner.flush_layout();
        owner.flush_paint();

        assert_eq!(
            mounts.get(),
            2,
            "the render object created on rebuild is mounted, as the initial tree is"
        );
    }
}
