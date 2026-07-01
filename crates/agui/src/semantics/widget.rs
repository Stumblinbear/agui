use bon::Builder;
use typed_floats::{Positive, PositiveFinite};

use crate::{
    prelude::{element::*, render_object::*},
    semantics::{Role, SemanticsConfig, SemanticsTreeBuilder},
};

/// A widget that annotates its subtree with semantics for assistive technology, without affecting
/// layout or paint. It contributes one node carrying the role, label, and merge behavior given here.
///
/// `merge_descendants` folds the whole subtree into this one node, the way a button absorbs its child
/// label. `excluded` drops the subtree from the semantics tree, for purely decorative content.
#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct Semantics<Child> {
    #[builder(finish_fn)]
    child: Child,

    role: Option<Role>,

    #[builder(into)]
    label: Option<String>,

    #[builder(default)]
    merge_descendants: bool,

    #[builder(default)]
    excluded: bool,
}

impl<Child> Widget for Semantics<Child>
where
    Child: Widget,
    Child::Render: RenderBox,
{
    type Element = SingleChildElement<Child::Element, RenderSemanticsAnnotations<Child::Render>>;

    type Render = RenderSemanticsAnnotations<Child::Render>;

    fn create(self, ctx: &mut CreateCtx) -> Self::Element {
        let Semantics {
            child,
            role,
            label,
            merge_descendants,
            excluded,
        } = self;

        let render = RenderSemanticsAnnotations {
            config: build_config(role, label, merge_descendants, excluded),
            semantics_id: None,
            size: Size::ZERO,
            child: RenderNode::new(()),
        };

        SingleChildElement::new(ctx, child, render)
    }

    fn update(self, ctx: &mut UpdateCtx, element: &mut Self::Element) {
        let Semantics {
            child,
            role,
            label,
            merge_descendants,
            excluded,
        } = self;

        element.render_object_mut().config = build_config(role, label, merge_descendants, excluded);
        ctx.mark_needs_semantics_update();

        element.update(ctx, child);
    }
}

fn build_config(
    role: Option<Role>,
    label: Option<String>,
    merge_descendants: bool,
    excluded: bool,
) -> SemanticsConfig {
    let mut node = accesskit::Node::new(role.unwrap_or(Role::Unknown));

    if let Some(label) = label {
        node.set_label(label);
    }

    SemanticsConfig {
        node,
        merge_descendants,
        excluded,
        ..Default::default()
    }
}

/// The render object of a [`Semantics`]: a passthrough box that lays out, paints, and hit-tests as its
/// child, and contributes the child's subtree under one semantic node carrying `config`.
pub struct RenderSemanticsAnnotations<Child: ?Sized> {
    config: SemanticsConfig,
    semantics_id: Option<SemanticsNodeId>,
    size: Size,
    child: RenderNode<Child>,
}

impl<Child: RenderBox + ?Sized> SingleChildRenderObject for RenderSemanticsAnnotations<Child> {
    type Child = Child;

    fn adopt_child(&mut self, child: MountedChild<Child>) {
        self.child.set(child);
    }
}

impl<Child: RenderBox + ?Sized> RenderObject for RenderSemanticsAnnotations<Child> {
    fn attach(&mut self, ctx: &mut UpdateCtx<'_>) {
        ctx.mark_needs_semantics_update();
    }

    fn detach(&mut self, ctx: &mut UpdateCtx<'_>) {
        ctx.mark_needs_semantics_update();
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .child(|d| self.child.describe(d))
            .finish()
    }
}

impl<Child: RenderBox + ?Sized> RenderBox for RenderSemanticsAnnotations<Child> {
    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.child.max_intrinsic_height(width)
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        self.child.measure(constraints)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.size = self.child.layout_and_get_size(ctx, constraints);
        self.size
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(constraints, baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        self.child.hit_test(result, position)
    }

    fn update_compositing_bits(&mut self) -> bool {
        self.child.update_compositing_bits()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.child.paint(ctx, offset);
    }

    fn build_semantics(&mut self, s: &mut SemanticsTreeBuilder<'_>) {
        s.node(
            &mut self.semantics_id,
            self.config.clone(),
            self.size,
            |s| {
                self.child.build_semantics(s);
            },
        );
    }
}
