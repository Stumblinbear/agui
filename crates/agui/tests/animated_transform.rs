use std::{rc::Rc, time::Duration};

use agui::{
    geometry::{Alignment, Offset, Size},
    input::hit_test::HitTestBehavior,
    paint::{
        command::PaintCommand,
        peniko::{Color, kurbo::Affine},
        scene::Scene,
    },
    render_object::box_layout::BoxConstraints,
    scheduling::Vsync,
    semantics::{Role, Semantics},
    widgets::{
        animated_transform::AnimatedTransform, listener::Listener,
        repaint_boundary::RepaintBoundary, sized_box::SizedBox,
    },
};
use agui_test::{
    ElementLifecycleCheck, Probe, WidgetTester,
    fixtures::{RecordingBox, TestBox},
};

/// The transform in effect at the single fill of a composed scene.
fn only_fill_transform(scene: &Scene) -> Affine {
    let flat = scene.flatten();

    let mut current = Affine::IDENTITY;
    let mut stack = Vec::new();
    for command in flat.commands() {
        match command {
            PaintCommand::PushTransform(transform) => {
                stack.push(current);
                current *= *transform;
            }
            PaintCommand::PopTransform => current = stack.pop().expect("balanced"),
            PaintCommand::Fill { .. } => return current,
            _ => {}
        }
    }

    panic!("expected a fill, got {:?}", flat.commands());
}

/// A leaf the transform paints under: a fixed-size box that draws a fill, so `only_fill_transform` can read
/// the transform in effect and a [`Probe`] can count how often it is repainted.
fn painted() -> TestBox {
    TestBox::new(Size::new(10, 10)).color(Color::BLACK)
}

#[test]
fn animated_transform_obeys_the_element_lifecycle() {
    ElementLifecycleCheck::new()
        .single_child(|child| AnimatedTransform::new(|_| Affine::IDENTITY).child(child));
}

/// Each tick resamples the transform and recomposites the retained layer at the new transform; the
/// subtree under it is painted once and replayed, not repainted.
#[test]
fn the_transform_recomposites_without_repainting_the_subtree() {
    let vsync = Vsync::new();
    let probe = Probe::new();

    let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
        .vsync(vsync.clone())
        .child(probe.wrap(painted()));

    let mut tester = WidgetTester::mount(widget);
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);
    assert_eq!(probe.paints(), 1);

    // The widget animates off this vsync, not the tester's clock, so ticking it advances the transform and the
    // pump flushes the resulting recomposite.
    vsync.tick(Duration::from_millis(16));
    tester.pump(Duration::ZERO);
    assert_eq!(
        only_fill_transform(&tester.composite_frame().rasterize()),
        Affine::translate((16.0, 0.0))
    );

    vsync.tick(Duration::from_millis(32));
    tester.pump(Duration::ZERO);
    assert_eq!(
        only_fill_transform(&tester.composite_frame().rasterize()),
        Affine::translate((32.0, 0.0))
    );

    assert_eq!(
        probe.paints(),
        1,
        "the subtree was painted once and replayed at each transform"
    );
}

/// Wrapping the subtree in a repaint boundary reuses its painting: the transform animates while the
/// subtree paints once.
#[test]
fn a_repaint_boundary_child_paints_once_across_the_animation() {
    let vsync = Vsync::new();
    let probe = Probe::new();

    let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
        .vsync(vsync.clone())
        .child(RepaintBoundary::new(probe.wrap(painted())));

    let mut tester = WidgetTester::mount(widget);
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);
    assert_eq!(probe.paints(), 1);

    vsync.tick(Duration::from_millis(16));
    tester.pump(Duration::ZERO);
    assert_eq!(
        only_fill_transform(&tester.composite_frame().rasterize()),
        Affine::translate((16.0, 0.0))
    );

    vsync.tick(Duration::from_millis(32));
    tester.pump(Duration::ZERO);
    assert_eq!(
        probe.paints(),
        1,
        "the boundary reused its painting across the animation"
    );
}

/// A hit is localized through the current transform, so a quarter-turn routes a point that lies only
/// within the rotated bounds to the child.
#[test]
fn a_hit_is_localized_through_the_current_transform() {
    use std::f64::consts::FRAC_PI_2;

    let widget = AnimatedTransform::new(|_| Affine::rotate(FRAC_PI_2)).child(
        Listener::builder()
            .behavior(HitTestBehavior::Opaque)
            .child(SizedBox::new().width(50).height(50)),
    );

    let mut tester = WidgetTester::mount(widget);
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);

    // A quarter-turn about the origin places the 50x50 child at x in [-50, 0]. The point (-5, 5) lies
    // outside the unrotated bounds but inside the rotated ones, localizing to the child.
    let result = tester.hit_test(Offset::new(-5.0, 5.0));

    assert!(!result.path().is_empty(), "the rotated child is hit");
}

/// Aligning the pivot to the child's center rotates it in place: a half-turn about the center keeps the
/// 50x50 child within its own bounds, so a far-corner hit still lands.
#[test]
fn the_pivot_aligns_to_the_child() {
    use std::f64::consts::PI;

    let widget = AnimatedTransform::new(|_| Affine::rotate(PI))
        .alignment(Alignment::CENTER)
        .child(
            Listener::builder()
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(50).height(50)),
        );

    let mut tester = WidgetTester::mount(widget);
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);

    // A half-turn about the center maps the child's (10, 10) to (40, 40); without the center pivot it
    // would map about the origin and leave the bounds entirely.
    let result = tester.hit_test(Offset::new(40.0, 40.0));

    assert!(!result.path().is_empty(), "the rotated child is hit");
}

/// Recompositing the moved layer marks the subtree's semantics even though nothing repaints, since the
/// subtree's geometry moved with it. The next flush re-walks the boundary.
#[test]
fn recompositing_marks_the_subtree_for_a_semantics_rewalk() {
    let render = RecordingBox::new();
    let builds = Rc::clone(&render.semantics_builds);

    let vsync = Vsync::new();
    let widget = AnimatedTransform::new(|now| Affine::translate((now.as_millis() as f64, 0.0)))
        .vsync(vsync.clone())
        .child(
            Semantics::new()
                .role(Role::Button)
                .label("Submit")
                .child(render),
        );

    let mut tester = WidgetTester::mount(widget);
    tester.resize_with(BoxConstraints::new(0, 100, 0, 100));
    tester.pump(Duration::ZERO);

    tester.flush_semantics();
    let walked = builds.get();
    tester.flush_semantics();
    assert_eq!(
        builds.get(),
        walked,
        "nothing changed, so the boundary is not re-walked"
    );

    // The recomposite marks the boundary through a deferred scope. The mark drains on the pump, and the flush
    // after re-walks it.
    vsync.tick(Duration::from_millis(16));
    tester.pump(Duration::ZERO);
    tester.flush_semantics();
    assert!(
        builds.get() > walked,
        "the recomposite marked the subtree for a re-walk"
    );
}
