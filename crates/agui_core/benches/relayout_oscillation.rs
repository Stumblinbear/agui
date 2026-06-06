//! Measures what a reversible relayout boundary's box/unbox churn would cost against the current
//! one-way boundary, for a node whose constraints flip tight and loose every frame.
//!
//! The current mechanism boxes a child the first time it is constrained tightly and leaves it boxed
//! forever, so every later frame dispatches through the erased `Rc<RefCell<dyn AnyRenderBox>>` form,
//! paying a `RefCell` borrow, an `Rc` deref, and a vtable call at each level. A reversible mechanism
//! would recover the child back to the inline, statically dispatched form whenever it is loosely
//! constrained again, so a loose frame dispatches concretely, but every tight-to-loose and
//! loose-to-tight edge pays a transition: an allocation and registry insert to box, a registry remove
//! and `Rc::try_unwrap` to unbox.
//!
//! Both forms are reconstructed standalone here as `depth` levels over a leaf. The bench drives a run
//! of layout frames that alternates tight and loose every frame and measures the total, so the
//! per-cycle transition churn of the reversible form competes against the inline-dispatch savings it
//! buys on loose frames. Paint is left out so the layout dispatch path carries the comparison without
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

/// Frames driven per oscillation run, alternating tight and loose so each run holds the same count of
/// each.
const FRAMES: usize = 64;

/// The cheap, allocation-free work each level does, identical on both paths so the only difference
/// measured is the dispatch path and the transition churn.
fn do_work(counter: &Cell<u64>) {
    counter.set(counter.get().wrapping_add(1));
}

/// The payload at one level of either chain: it counts on layout and on paint, the cheap per-level
/// work both dispatch paths carry.
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

/// One level of a chain, holding its payload either inline or boxed-and-registered, the two forms a
/// relayout boundary moves between.
enum Level {
    /// The payload held by value, dispatched statically.
    Inline(Payload),

    /// The payload shared behind an `Rc<RefCell>` and registered as a boundary, dispatched through the
    /// erased form. `boundary` is the scope that unregisters it.
    Boxed {
        content: Rc<RefCell<Payload>>,
        boundary: LayoutScope,
    },
}

impl Level {
    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) {
        match self {
            Level::Inline(payload) => {
                payload.layout(ctx, constraints);
            }
            Level::Boxed { content, .. } => {
                content.borrow_mut().layout(ctx, constraints);
            }
        }
    }

    /// Boxes an inline level: allocates the shared cell and registers it as a boundary under `scope`.
    fn box_and_register(&mut self, scope: &LayoutScope, paint: &PaintScope) {
        let Level::Inline(payload) = self else {
            return;
        };

        let payload = std::mem::replace(payload, dummy_payload());

        let content = Rc::new(RefCell::new(payload));
        let boundary = scope.register(erase(Rc::clone(&content)), paint.clone());

        *self = Level::Boxed { content, boundary };
    }

    /// Unboxes a registered level: unregisters the boundary and recovers the payload by value.
    fn unbox(&mut self) {
        let Level::Boxed { boundary, .. } = self else {
            return;
        };

        boundary.unregister();

        let this = std::mem::replace(self, Level::Inline(dummy_payload()));
        let Level::Boxed { content, boundary } = this else {
            unreachable!("the boxed form was just observed");
        };

        match Rc::try_unwrap(content) {
            Ok(cell) => *self = Level::Inline(cell.into_inner()),
            Err(content) => *self = Level::Boxed { content, boundary },
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

/// The one-way run: every level is boxed once and stays boxed, so every frame dispatches through the
/// erased form regardless of whether it is tight or loose. No per-cycle transition is paid.
fn run_one_way(levels: &mut [Level], leaf: &mut Payload) {
    for f in 0..FRAMES {
        let constraints = if f % 2 == 0 { tight() } else { loose() };

        let mut ctx = LayoutCtx::detached();
        for level in levels.iter_mut() {
            level.layout(&mut ctx, constraints);
        }
        black_box(leaf.layout(&mut ctx, constraints));
    }
}

/// The reversible run: each tight frame boxes every level and dispatches boxed; each loose frame
/// unboxes every level and dispatches inline. Every frame pays a full transition across the chain.
fn run_reversible(levels: &mut [Level], leaf: &mut Payload, h: &Harness) {
    for f in 0..FRAMES {
        let is_tight = f % 2 == 0;

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

/// Builds `depth` independent inline levels plus a leaf, the shape both runs walk.
fn build_levels(depth: usize, counter: &Rc<Cell<u64>>) -> (Vec<Level>, Payload) {
    let levels = (0..depth)
        .map(|_| {
            Level::Inline(Payload {
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
    let mut group = c.benchmark_group("relayout_oscillation");
    group.measurement_time(std::time::Duration::from_secs(2));

    let depths = [1usize, 4, 16];

    for &depth in &depths {
        let counter = Rc::new(Cell::new(0));

        // One-way: box every level once up front, then leave it boxed for the whole run.
        group.bench_with_input(BenchmarkId::new("one_way", depth), &depth, |b, &depth| {
            b.iter_batched_ref(
                || {
                    let h = harness(&counter);
                    let (mut levels, leaf) = build_levels(depth, &counter);
                    for level in &mut levels {
                        level.box_and_register(&h.scope, &h.paint);
                    }
                    (h, levels, leaf)
                },
                |(_h, levels, leaf)| run_one_way(levels, leaf),
                criterion::BatchSize::SmallInput,
            );
        });

        // Reversible: start inline, then flip every level every frame.
        group.bench_with_input(
            BenchmarkId::new("reversible", depth),
            &depth,
            |b, &depth| {
                b.iter_batched_ref(
                    || {
                        let h = harness(&counter);
                        let (levels, leaf) = build_levels(depth, &counter);
                        (h, levels, leaf)
                    },
                    |(h, levels, leaf)| run_reversible(levels, leaf, h),
                    criterion::BatchSize::SmallInput,
                );
            },
        );

        black_box(counter.get());
    }

    group.finish();

    // The isolated cost of a single box transition and a single unbox transition on one level, so the
    // per-cycle churn the runs above pay can be read directly.
    let mut tx = c.benchmark_group("relayout_transition");
    tx.measurement_time(std::time::Duration::from_secs(2));

    let counter = Rc::new(Cell::new(0));

    tx.bench_function("box_transition", |b| {
        b.iter_batched_ref(
            || {
                let h = harness(&counter);
                let level = Level::Inline(Payload {
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
                let mut level = Level::Inline(Payload {
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
