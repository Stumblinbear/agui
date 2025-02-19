use bon::Builder;
use typed_floats::{as_const, Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{LayoutCtx, MessageCtx, UpdateCtx},
    edge_insets::EdgeInsetsGeometry,
    hit_test::HitTestResult,
    offset::Offset,
    size::Size,
    text_baseline::TextBaseline,
    text_direction::TextDirection,
    tree::{Tree, TreeState},
    view::{View, ViewDraw, ViewLayout, ViewLifecycle},
    view_id::ViewId,
};

#[derive(Builder)]
#[builder(start_fn = new)]
#[builder(finish_fn = child)]
pub struct Padding<EdgeGeometry, Child> {
    #[builder(start_fn)]
    padding: EdgeGeometry,

    #[builder(finish_fn)]
    child: Child,

    #[builder(default)]
    text_direction: TextDirection,
}

#[derive(Default)]
struct State {
    child_offset: Offset,
}

const CHILD_ID: ViewId = ViewId::new(0);

impl<EdgeGeometry, Child> ViewLifecycle for Padding<EdgeGeometry, Child>
where
    EdgeGeometry: EdgeInsetsGeometry,
    Child: ViewLifecycle,
{
    fn state(&self) -> TreeState {
        TreeState::new(State::default())
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

impl<EdgeGeometry, Child> ViewLayout for Padding<EdgeGeometry, Child>
where
    EdgeGeometry: EdgeInsetsGeometry,
    Child: ViewLayout,
{
    fn min_intrinsic_width(
        &self,
        tree: &Tree,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_height = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(height - self.padding.vertical())
                    .get(),
            )
        };

        self.child
            .min_intrinsic_width(tree.child(CHILD_ID), inner_height)
            .map(|width| {
                PositiveFinite::try_from(width + self.padding.horizontal())
                    .expect("minimum intrinsic width of padding must be finite")
            })
    }

    fn max_intrinsic_width(
        &self,
        tree: &Tree,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_height = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(height - self.padding.vertical())
                    .get(),
            )
        };

        self.child
            .max_intrinsic_width(tree.child(CHILD_ID), inner_height)
            .map(|width| {
                PositiveFinite::try_from(width + self.padding.horizontal())
                    .expect("minimum intrinsic width of padding must be finite")
            })
    }

    fn min_intrinsic_height(
        &self,
        tree: &Tree,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_width = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(width - self.padding.horizontal())
                    .get(),
            )
        };

        self.child
            .min_intrinsic_height(tree.child(CHILD_ID), inner_width)
            .map(|height| {
                PositiveFinite::try_from(height + self.padding.vertical())
                    .expect("minimum intrinsic height of padding must be finite")
            })
    }

    fn max_intrinsic_height(
        &self,
        tree: &Tree,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        let inner_width = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(width - self.padding.horizontal())
                    .get(),
            )
        };

        self.child
            .max_intrinsic_height(tree.child(CHILD_ID), inner_width)
            .map(|height| {
                PositiveFinite::try_from(height + self.padding.vertical())
                    .expect("minimum intrinsic height of padding must be finite")
            })
    }

    fn measure(&self, tree: &Tree, constraints: Constraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);

        let child_size = self.child.measure(tree.child(CHILD_ID), inner_constraints);

        constraints
            .constrain(Size::new(self.padding.horizontal(), self.padding.vertical()) + child_size)
    }

    fn layout(&self, mut ctx: LayoutCtx, constraints: Constraints) {
        let inner_constraints = constraints.deflate(&self.padding);

        let child_size = ctx
            .child(CHILD_ID, |ctx| self.child.layout(ctx, inner_constraints))
            .size();

        ctx.state_mut::<State>().child_offset =
            Offset::new(self.padding.left(self.text_direction), self.padding.top());

        ctx.size = constraints
            .constrain(Size::new(self.padding.horizontal(), self.padding.vertical()) + child_size);
    }

    fn measure_baseline(
        &self,
        tree: &Tree,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let inner_constraints = constraints.deflate(&self.padding);

        self.child
            .measure_baseline(tree.child(CHILD_ID), inner_constraints, baseline)
            .map(|baseline| {
                PositiveFinite::try_from(baseline + self.padding.top())
                    .expect("baseline of padding must be finite")
            })
    }

    fn distance_to_baseline(
        &self,
        tree: &mut Tree,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child
            .distance_to_baseline(tree.child_mut(CHILD_ID), baseline)
            .map(|distance| {
                PositiveFinite::try_from(distance + tree.state::<State>().child_offset.y)
                    .expect("distance to baseline of padding was not a positive finite number")
            })
    }

    fn hit_test(&self, tree: &Tree, result: &mut HitTestResult, position: Offset) -> bool {
        if !tree.size.contains(position) {
            return false;
        }

        result.with_offset(
            tree.state::<State>().child_offset,
            position,
            |result, transformed| {
                self.child
                    .hit_test(tree.child(CHILD_ID), result, transformed)
            },
        )
    }
}
impl<Renderer, EdgeGeometry, Child> ViewDraw<Renderer> for Padding<EdgeGeometry, Child>
where
    Renderer: crate::renderer::Renderer,
    EdgeGeometry: EdgeInsetsGeometry,
    Child: View<Renderer>,
{
    fn draw(&self, tree: &mut Tree, renderer: &mut Renderer) {
        let left = self.padding.left(self.text_direction);
        let top = self.padding.top();

        renderer.with_offset(Offset::new(left, top), |renderer| {
            self.child.draw(tree.child_mut(CHILD_ID), renderer);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{edge_insets::EdgeInsets, widgets::sized_box::SizedBox};

    #[test]
    fn padding() {
        let mut tree = Tree::empty();

        let padding = Padding::new(EdgeInsets::all(10.0)).child(());
        tree.update(&padding);
        padding.layout(LayoutCtx::new(&mut tree), Constraints::new(0, 128, 0, 128));

        assert_eq!(tree.size, Size::new(20.0, 20.0));

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::shrink());
        tree.update(&padding);
        padding.layout(LayoutCtx::new(&mut tree), Constraints::new(0, 128, 0, 128));

        assert_eq!(tree.size, Size::new(100.0, 100.0));

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::expand());
        tree.update(&padding);
        padding.layout(LayoutCtx::new(&mut tree), Constraints::new(0, 128, 0, 128));

        assert_eq!(tree.size, Size::new(128.0, 128.0));
    }
}
