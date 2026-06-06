//! Measures what the relayout boundary's box/unbox handling costs for a node whose constraints flip
//! tight and loose, comparing the current hysteresis behavior against the per-flip baseline it replaces
//! and against a naive full-reversal that boxes and unboxes on every flip.
//!
//! A child is boxed and registered the first time it is constrained tightly. The baseline drops the
//! registration on a loose frame and restores it on a tight frame, leaving the child boxed across the
//! whole run, so every frame dispatches through the erased `Rc<RefCell<dyn AnyRenderBox>>` form. The
//! hysteresis behavior matches that per-flip register and unregister, and additionally recovers the
//! child to the inline, statically dispatched form once it has stayed loose for a sustained run, so a
//! settled-loose node dispatches concretely on its long loose tail. A naive full-reversal instead boxes
//! and unboxes on every flip, paying an allocation and registry insert to box and a registry remove and
//! `Rc::try_unwrap` to unbox at every edge.
//!
//! The level state machine here mirrors the crate's `RelayoutRenderNode::reshape`; the real node is
//! private and requires a mounted holder under a registered scope per level, so reconstructing the
//! transitions standalone keeps all arms on identical infrastructure and the comparison fair. The crate
//! tests prove the real node takes these transitions. All arms walk the same `depth` levels over a leaf
//! and drive the same frame schedule, so the only difference measured is the dispatch path and the
//! transition churn. Paint is left out so the layout dispatch path carries the comparison without
//! per-frame scene allocation drowning it. The isolated cost of one box transition and one unbox
//! transition is measured separately so the churn can be read directly.

use std::{cell::Cell, cell::RefCell, rc::Rc};

use agui_core::{
    context::{LayoutCtx, MountCtx, PaintCtx},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    pipeline::{
        layout::{BoundaryContent, LayoutPipeline, LayoutScope},
        paint::PaintScope,
    },
    render_object::{
        RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    text::TextBaseline,
};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

/// Frames driven per fast-oscillation run, alternating tight and loose so each run holds the same count
/// of each.
const OSCILLATION_FRAMES: usize = 64;

/// Tight frames that open a settle-into-loose run before the long loose tail.
const SETTLE_TIGHT_FRAMES: usize = 2;

/// Loose frames in the tail of a settle-into-loose run, far more than the unbox threshold so the
/// hysteresis arm recovers early and dispatches inline over the rest.
const SETTLE_LOOSE_FRAMES: usize = 256;

/// Loose layouts a registered level survives in a row before the hysteresis arm recovers it, matching
/// the crate's `RelayoutRenderNode::UNBOX_AFTER_LOOSE_LAYOUTS`.
const UNBOX_AFTER_LOOSE_LAYOUTS: u32 = 32;

/// The cheap, allocation-free work each level does, identical on every path so the only difference
/// measured is the dispatch path and the transition churn.
fn do_work(counter: &Cell<u64>) {
    counter.set(counter.get().wrapping_add(1));
}

/// The payload at one level of the chain: it counts on layout and on paint, the cheap per-level work
/// every dispatch path carries.
struct Payload {
    counter: Rc<Cell<u64>>,
}

impl RenderObject for Payload {
    fn mount(&mut self, _: &mut MountCtx) {}
    fn unmount(&mut self, _: &mut MountCtx) {}
    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

impl RenderBox for Payload {
    fn min_intrinsic_width(
        &self,
        _: typed_floats::Positive<f32>,
    ) -> Option<typed_floats::PositiveFinite<f32>> {
        None
    }
    fn max_intrinsic_width(
        &self,
        _: typed_floats::Positive<f32>,
    ) -> Option<typed_floats::PositiveFinite<f32>> {
        None
    }
    fn min_intrinsic_height(
        &self,
        _: typed_floats::Positive<f32>,
    ) -> Option<typed_floats::PositiveFinite<f32>> {
        None
    }
    fn max_intrinsic_height(
        &self,
        _: typed_floats::Positive<f32>,
    ) -> Option<typed_floats::PositiveFinite<f32>> {
        None
    }
    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.smallest()
    }
    fn layout(&mut self, _: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        do_work(&self.counter);
        constraints.smallest()
    }
    fn measure_baseline(
        &self,
        _: BoxConstraints,
        _: TextBaseline,
    ) -> Option<typed_floats::PositiveFinite<f32>> {
        None
    }
    fn distance_to_baseline(
        &mut self,
        _: TextBaseline,
    ) -> Option<typed_floats::PositiveFinite<f32>> {
        None
    }
    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }
    fn paint(&mut self, _: &mut PaintCtx, _: Offset) {
        do_work(&self.counter);
    }
}

/// The payload of one level, held either inline or shared behind the boundary cell, the two forms a
/// relayout boundary moves between.
enum Form {
    /// The payload held by value, dispatched statically.
    Inline(Payload),

    /// The payload shared behind an `Rc<RefCell>`, dispatched through the erased form. `boundary` is
    /// `Some` while it is registered as a relayout boundary and `None` while it is loosely constrained.
    Boxed {
        content: Rc<RefCell<Payload>>,
        boundary: Option<LayoutScope>,
    },
}

/// One level of a chain, tracking its boundary form and how long it has been loosely constrained, so it
/// can take the same transitions the crate's relayout boundary takes.
struct Level {
    form: Form,
    loose_streak: u32,
}

impl Level {
    fn inline(payload: Payload) -> Self {
        Self {
            form: Form::Inline(payload),
            loose_streak: 0,
        }
    }

    fn is_inline(&self) -> bool {
        matches!(self.form, Form::Inline(_))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) {
        match &mut self.form {
            Form::Inline(payload) => {
                payload.layout(ctx, constraints);
            }
            Form::Boxed { content, .. } => {
                content.borrow_mut().layout(ctx, constraints);
            }
        }
    }

    /// Boxes an inline level: allocates the shared cell and registers it as a boundary under `scope`.
    fn box_and_register(&mut self, scope: &LayoutScope, paint: &PaintScope) {
        let Form::Inline(payload) = &mut self.form else {
            return;
        };

        let payload = std::mem::replace(payload, dummy_payload());

        let content = Rc::new(RefCell::new(payload));
        let boundary = scope.register(erase(Rc::clone(&content)), paint.clone());

        self.form = Form::Boxed {
            content,
            boundary: Some(boundary),
        };
    }

    /// Registers an already boxed level as a boundary again under `scope`.
    fn register(&mut self, scope: &LayoutScope, paint: &PaintScope) {
        if let Form::Boxed { content, boundary } = &mut self.form {
            *boundary = Some(scope.register(erase(Rc::clone(content)), paint.clone()));
        }
    }

    /// Drops the boundary registration of a boxed level, leaving it boxed but unregistered.
    fn unregister(&mut self) {
        if let Form::Boxed { boundary, .. } = &mut self.form
            && let Some(boundary) = boundary.take()
        {
            boundary.unregister();
        }
    }

    /// Unboxes a level: unregisters any boundary and recovers the payload by value.
    fn unbox(&mut self) {
        self.unregister();

        let this = std::mem::replace(&mut self.form, Form::Inline(dummy_payload()));
        let Form::Boxed { content, boundary } = this else {
            return;
        };

        match Rc::try_unwrap(content) {
            Ok(cell) => self.form = Form::Inline(cell.into_inner()),
            Err(content) => self.form = Form::Boxed { content, boundary },
        }
    }

    /// Takes the transition the crate's relayout boundary takes for `tight`, tracking the loose streak
    /// and recovering to inline only after a sustained loose run.
    fn reshape_hysteresis(&mut self, tight: bool, scope: &LayoutScope, paint: &PaintScope) {
        if tight {
            self.loose_streak = 0;
        } else {
            self.loose_streak = self.loose_streak.saturating_add(1);
        }

        match &self.form {
            Form::Inline(_) if tight => self.box_and_register(scope, paint),
            Form::Boxed { boundary: None, .. } if tight => self.register(scope, paint),
            Form::Boxed {
                boundary: Some(_), ..
            } if !tight => self.unregister(),
            Form::Boxed { boundary: None, .. }
                if self.loose_streak >= UNBOX_AFTER_LOOSE_LAYOUTS =>
            {
                self.unbox()
            }
            _ => {}
        }
    }

    /// Takes the baseline transition for `tight`: register on tight, unregister on loose, never
    /// recovering to inline, so a boxed level stays boxed for the whole run.
    fn reshape_baseline(&mut self, tight: bool, scope: &LayoutScope, paint: &PaintScope) {
        match &self.form {
            Form::Inline(_) if tight => self.box_and_register(scope, paint),
            Form::Boxed { boundary: None, .. } if tight => self.register(scope, paint),
            Form::Boxed {
                boundary: Some(_), ..
            } if !tight => self.unregister(),
            _ => {}
        }
    }
}

/// A payload that counts nowhere, used only to vacate a slot while its real payload is moved out.
fn dummy_payload() -> Payload {
    Payload {
        counter: Rc::new(Cell::new(0)),
    }
}

/// Erases a shared payload into the form the boundary registry holds.
fn erase(content: Rc<RefCell<Payload>>) -> BoundaryContent {
    content
}

/// A pipeline and the root scope its levels register under, kept alive for the run.
struct Harness {
    _pipeline: LayoutPipeline,
    scope: LayoutScope,
    paint: PaintScope,
}

fn harness(counter: &Rc<Cell<u64>>) -> Harness {
    let root: BoundaryContent = Rc::new(RefCell::new(Payload {
        counter: Rc::clone(counter),
    }));
    let (pipeline, scope) = LayoutPipeline::new(root, PaintScope::detached());

    Harness {
        _pipeline: pipeline,
        scope,
        paint: PaintScope::detached(),
    }
}

fn tight() -> BoxConstraints {
    BoxConstraints::tight(Size::new(10.0, 10.0))
}

fn loose() -> BoxConstraints {
    BoxConstraints::new(0, 10, 0, 10)
}

/// The schedule one run drives: whether each frame is tight, in order.
fn oscillation_schedule() -> Vec<bool> {
    (0..OSCILLATION_FRAMES).map(|f| f % 2 == 0).collect()
}

fn settle_schedule() -> Vec<bool> {
    let mut schedule = vec![true; SETTLE_TIGHT_FRAMES];
    schedule.extend(std::iter::repeat_n(false, SETTLE_LOOSE_FRAMES));
    schedule
}

/// Runs a schedule with the baseline behavior: register on tight, unregister on loose, leaving every
/// level boxed for the whole run.
fn run_baseline(levels: &mut [Level], leaf: &mut Payload, h: &Harness, schedule: &[bool]) {
    for &is_tight in schedule {
        for level in levels.iter_mut() {
            level.reshape_baseline(is_tight, &h.scope, &h.paint);
        }

        let constraints = if is_tight { tight() } else { loose() };

        let mut ctx = LayoutCtx::detached();
        for level in levels.iter_mut() {
            level.layout(&mut ctx, constraints);
        }
        black_box(leaf.layout(&mut ctx, constraints));
    }
}

/// Runs a schedule with the hysteresis behavior: register and unregister per flip, recovering a level
/// to inline once it has stayed loose past the threshold.
fn run_hysteresis(levels: &mut [Level], leaf: &mut Payload, h: &Harness, schedule: &[bool]) {
    for &is_tight in schedule {
        for level in levels.iter_mut() {
            level.reshape_hysteresis(is_tight, &h.scope, &h.paint);
        }

        let constraints = if is_tight { tight() } else { loose() };

        let mut ctx = LayoutCtx::detached();
        for level in levels.iter_mut() {
            level.layout(&mut ctx, constraints);
        }
        black_box(leaf.layout(&mut ctx, constraints));
    }
}

/// Runs a schedule boxing on every tight frame and unboxing on every loose frame, paying a full
/// transition across the chain at each edge.
fn run_naive_reversible(levels: &mut [Level], leaf: &mut Payload, h: &Harness, schedule: &[bool]) {
    for &is_tight in schedule {
        if is_tight {
            for level in levels.iter_mut() {
                level.box_and_register(&h.scope, &h.paint);
            }
        } else {
            for level in levels.iter_mut() {
                level.unbox();
            }
        }

        let constraints = if is_tight { tight() } else { loose() };

        let mut ctx = LayoutCtx::detached();
        for level in levels.iter_mut() {
            level.layout(&mut ctx, constraints);
        }
        black_box(leaf.layout(&mut ctx, constraints));
    }
}

/// Builds `depth` independent inline levels plus a leaf, the shape every run walks.
fn build_levels(depth: usize, counter: &Rc<Cell<u64>>) -> (Vec<Level>, Payload) {
    let levels = (0..depth)
        .map(|_| {
            Level::inline(Payload {
                counter: Rc::clone(counter),
            })
        })
        .collect();

    let leaf = Payload {
        counter: Rc::clone(counter),
    };

    (levels, leaf)
}

fn bench_oscillation(c: &mut Criterion) {
    let depths = [1usize, 4, 16];

    let mut osc = c.benchmark_group("relayout_oscillation");
    osc.measurement_time(std::time::Duration::from_secs(2));

    let schedule = oscillation_schedule();

    for &depth in &depths {
        let counter = Rc::new(Cell::new(0));

        // Baseline: box once, then register on tight and unregister on loose every flip, staying boxed.
        osc.bench_with_input(
            BenchmarkId::new("baseline_one_way", depth),
            &depth,
            |b, &depth| {
                b.iter_batched_ref(
                    || {
                        let h = harness(&counter);
                        let (mut levels, leaf) = build_levels(depth, &counter);
                        for level in &mut levels {
                            level.box_and_register(&h.scope, &h.paint);
                        }
                        (h, levels, leaf)
                    },
                    |(h, levels, leaf)| run_baseline(levels, leaf, h, &schedule),
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Hysteresis: start inline and take the crate's transitions; a fast flip never reaches unbox.
        osc.bench_with_input(
            BenchmarkId::new("hysteresis", depth),
            &depth,
            |b, &depth| {
                b.iter_batched_ref(
                    || {
                        let h = harness(&counter);
                        let (levels, leaf) = build_levels(depth, &counter);
                        (h, levels, leaf)
                    },
                    |(h, levels, leaf)| run_hysteresis(levels, leaf, h, &schedule),
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        // Naive reversible: box and unbox every flip, the regression reference.
        osc.bench_with_input(
            BenchmarkId::new("naive_reversible", depth),
            &depth,
            |b, &depth| {
                b.iter_batched_ref(
                    || {
                        let h = harness(&counter);
                        let (levels, leaf) = build_levels(depth, &counter);
                        (h, levels, leaf)
                    },
                    |(h, levels, leaf)| run_naive_reversible(levels, leaf, h, &schedule),
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        black_box(counter.get());
    }

    osc.finish();

    let mut settle = c.benchmark_group("relayout_settle_loose");
    settle.measurement_time(std::time::Duration::from_secs(2));

    let schedule = settle_schedule();

    for &depth in &depths {
        let counter = Rc::new(Cell::new(0));

        // Baseline: stays boxed across the whole loose tail, dispatching boxed every frame.
        settle.bench_with_input(BenchmarkId::new("baseline", depth), &depth, |b, &depth| {
            b.iter_batched_ref(
                || {
                    let h = harness(&counter);
                    let (mut levels, leaf) = build_levels(depth, &counter);
                    for level in &mut levels {
                        level.box_and_register(&h.scope, &h.paint);
                    }
                    (h, levels, leaf)
                },
                |(h, levels, leaf)| run_baseline(levels, leaf, h, &schedule),
                criterion::BatchSize::SmallInput,
            );
        });

        // Hysteresis: recovers to inline after the threshold, dispatching inline over the rest of the
        // tail.
        settle.bench_with_input(
            BenchmarkId::new("hysteresis", depth),
            &depth,
            |b, &depth| {
                b.iter_batched_ref(
                    || {
                        let h = harness(&counter);
                        let (levels, leaf) = build_levels(depth, &counter);
                        (h, levels, leaf)
                    },
                    |(h, levels, leaf)| {
                        run_hysteresis(levels, leaf, h, &schedule);
                        // Confirm the tail actually ran inline, so the arm measures the recovered path.
                        debug_assert!(levels.iter().all(Level::is_inline));
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        black_box(counter.get());
    }

    settle.finish();

    // The isolated cost of a single box transition and a single unbox transition on one level, so the
    // per-cycle churn the runs above pay can be read directly.
    let mut tx = c.benchmark_group("relayout_transition");
    tx.measurement_time(std::time::Duration::from_secs(2));

    let counter = Rc::new(Cell::new(0));

    tx.bench_function("box_transition", |b| {
        b.iter_batched_ref(
            || {
                let h = harness(&counter);
                let level = Level::inline(Payload {
                    counter: Rc::clone(&counter),
                });
                (h, level)
            },
            |(h, level)| level.box_and_register(&h.scope, &h.paint),
            criterion::BatchSize::SmallInput,
        );
    });

    tx.bench_function("unbox_transition", |b| {
        b.iter_batched_ref(
            || {
                let h = harness(&counter);
                let mut level = Level::inline(Payload {
                    counter: Rc::clone(&counter),
                });
                level.box_and_register(&h.scope, &h.paint);
                (h, level)
            },
            |(_h, level)| level.unbox(),
            criterion::BatchSize::SmallInput,
        );
    });

    tx.finish();
}

criterion_group!(benches, bench_oscillation);
criterion_main!(benches);
