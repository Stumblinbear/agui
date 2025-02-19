use bon::Builder;
use typed_floats::{as_const, Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{LayoutCtx, MessageCtx, UpdateCtx},
    hit_test::HitTestResult,
    offset::Offset,
    size::Size,
    text_baseline::TextBaseline,
    tree::{Tree, TreeState},
    view::{View, ViewDraw, ViewLayout, ViewLifecycle},
    view_id::ViewId,
};

#[derive(Builder)]
pub struct SizedBox<Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: Child,
}

impl SizedBox<()> {
    pub fn new(width: Option<Positive<f32>>, height: Option<Positive<f32>>) -> Self {
        SizedBox {
            width,
            height,

            child: (),
        }
    }

    pub fn shrink() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, 0.0)),
            height: Some(as_const!(Positive, f32, 0.0)),

            child: (),
        }
    }

    pub fn expand() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: (),
        }
    }

    pub fn child<Child>(self, child: Child) -> SizedBox<Child> {
        SizedBox {
            width: self.width,
            height: self.height,

            child,
        }
    }
}

impl From<Size> for SizedBox<()> {
    fn from(size: Size) -> Self {
        Self {
            width: Some(Positive::<f32>::try_from(size.width).expect("width must be positive")),
            height: Some(Positive::<f32>::try_from(size.height).expect("height must be positive")),

            child: (),
        }
    }
}

impl<Child> SizedBox<Child> {
    fn additional_constraints(&self) -> Constraints {
        let mut constraints = Constraints::default();

        if let Some(width) = self.width {
            constraints = constraints.tighten_width(width.get());
        }

        if let Some(height) = self.height {
            constraints = constraints.tighten_height(height.get());
        }

        constraints
    }
}

const CHILD_ID: ViewId = ViewId::new(0);

impl<Child> ViewLifecycle for SizedBox<Child>
where
    Child: ViewLifecycle,
{
    fn state(&self) -> TreeState {
        TreeState::none()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn update(&self, mut ctx: UpdateCtx) {
        ctx.child(CHILD_ID, |ctx| self.child.update(ctx));
    }

    fn message(&self, ctx: MessageCtx) {
        match ctx.routing_id() {
            Some(CHILD_ID) => self.child.message(ctx),
            _ => unreachable!(),
        }
    }
}

impl<Child> ViewLayout for SizedBox<Child>
where
    Child: ViewLayout,
{
    fn min_intrinsic_width(
        &self,
        tree: &Tree,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.width
            .and_then(|width| PositiveFinite::try_from(width).ok())
            .or_else(|| self.child.min_intrinsic_width(tree.child(CHILD_ID), height))
    }

    fn max_intrinsic_width(
        &self,
        tree: &Tree,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.width
            .and_then(|width| PositiveFinite::try_from(width).ok())
            .or_else(|| self.child.max_intrinsic_width(tree.child(CHILD_ID), height))
    }

    fn min_intrinsic_height(
        &self,
        tree: &Tree,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.height
            .and_then(|height| PositiveFinite::try_from(height).ok())
            .or_else(|| self.child.min_intrinsic_height(tree.child(CHILD_ID), width))
    }

    fn max_intrinsic_height(
        &self,
        tree: &Tree,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.height
            .and_then(|height| PositiveFinite::try_from(height).ok())
            .or_else(|| self.child.max_intrinsic_height(tree.child(CHILD_ID), width))
    }

    fn measure(&self, tree: &Tree, constraints: Constraints) -> Size {
        self.child
            .measure(tree, self.additional_constraints().enforce(constraints))
    }

    fn layout(&self, mut ctx: LayoutCtx, constraints: Constraints) {
        ctx.size = ctx
            .child(CHILD_ID, |ctx| {
                self.child
                    .layout(ctx, self.additional_constraints().enforce(constraints))
            })
            .size();
    }

    fn measure_baseline(
        &self,
        tree: &Tree,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child.measure_baseline(
            tree.child(CHILD_ID),
            self.additional_constraints().enforce(constraints),
            baseline,
        )
    }

    fn distance_to_baseline(
        &self,
        tree: &mut Tree,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child
            .distance_to_baseline(tree.child_mut(CHILD_ID), baseline)
    }

    fn hit_test(&self, tree: &Tree, result: &mut HitTestResult, position: Offset) -> bool {
        if !tree.size.contains(position) {
            return false;
        }

        self.child.hit_test(tree.child(CHILD_ID), result, position)
    }
}
impl<Renderer, Child> ViewDraw<Renderer> for SizedBox<Child>
where
    Renderer: crate::renderer::Renderer,
    Child: View<Renderer>,
{
    fn draw(&self, tree: &mut Tree, renderer: &mut Renderer) {
        self.child.draw(tree.child_mut(CHILD_ID), renderer);
    }
}
