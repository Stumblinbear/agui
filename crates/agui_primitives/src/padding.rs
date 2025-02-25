use bon::Builder;
use typed_floats::{as_const, Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    edge_insets::{EdgeInsets, EdgeInsetsGeometry},
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    render_object::RenderObject,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    text_direction::TextDirection,
    view::View,
};

#[derive(Builder)]
#[builder(finish_fn = child)]
pub struct Padding<EdgeGeometry, Child> {
    #[builder(finish_fn)]
    child: Child,

    padding: EdgeGeometry,

    #[builder(default)]
    text_direction: TextDirection,
}

impl<EdgeGeometry, Child> Padding<EdgeGeometry, Child> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        padding: EdgeGeometry,
    ) -> PaddingBuilder<EdgeGeometry, Child, padding_builder::SetPadding> {
        Self::builder().padding(padding)
    }
}

impl<EdgeGeometry, Child> View for Padding<EdgeGeometry, Child>
where
    EdgeGeometry: EdgeInsetsGeometry,
    Child: View,
{
    type Render = RenderPadding<Child::Render>;

    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child_mut(0, &self.child).update(&old.child, ctx);
    }

    fn message(&self, element: &mut Element, ctx: MessageCtx) {
        match ctx.routing_id() {
            Some(0) => element.child_mut(0, &self.child).message(ctx),
            _ => unreachable!(),
        }
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        RenderPadding {
            padding: EdgeInsets {
                left: self.padding.left(self.text_direction),
                top: self.padding.top(),
                right: self.padding.right(self.text_direction),
                bottom: self.padding.bottom(),
            },

            child: element.child(0, &self.child).create_render_object(),
            child_offset: Offset::ZERO,

            size: Size::ZERO,
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        let padding = EdgeInsets {
            left: self.padding.left(self.text_direction),
            top: self.padding.top(),
            right: self.padding.right(self.text_direction),
            bottom: self.padding.bottom(),
        };

        // TODO(trevin): mark for re-layout if padding changes
        render_object.padding = padding;

        element
            .child(0, &self.child)
            .update_render_object(&mut render_object.child);
    }
}

pub struct RenderPadding<Child> {
    padding: EdgeInsets,

    child: Child,
    child_offset: Offset,

    size: Size,
}

impl<Child> RenderObject for RenderPadding<Child>
where
    Child: RenderObject,
{
    type Width = Child::Width;
    type Height = Child::Height;

    type WidthIntrinsic = Child::WidthIntrinsic;
    type HeightIntrinsic = Child::HeightIntrinsic;

    fn mount(&mut self, _: &mut UpdateCtx) {}

    fn unmount(&mut self, _: &mut UpdateCtx) {}

    fn size(&self) -> Size {
        self.size
    }

    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let inner_height = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(height - self.padding.vertical())
                    .get(),
            )
        };

        self.child.min_intrinsic_width(inner_height).map(|width| {
            PositiveFinite::try_from(width + self.padding.horizontal())
                .expect("minimum intrinsic width of padding must be finite")
        })
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let inner_height = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(height - self.padding.vertical())
                    .get(),
            )
        };

        self.child.max_intrinsic_width(inner_height).map(|width| {
            PositiveFinite::try_from(width + self.padding.horizontal())
                .expect("minimum intrinsic width of padding must be finite")
        })
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let inner_width = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(width - self.padding.horizontal())
                    .get(),
            )
        };

        self.child.min_intrinsic_height(inner_width).map(|height| {
            PositiveFinite::try_from(height + self.padding.vertical())
                .expect("minimum intrinsic height of padding must be finite")
        })
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let inner_width = unsafe {
            Positive::<f32>::new_unchecked(
                as_const!(NonNaN, f32, 0.0)
                    .max(width - self.padding.horizontal())
                    .get(),
            )
        };

        self.child.max_intrinsic_height(inner_width).map(|height| {
            PositiveFinite::try_from(height + self.padding.vertical())
                .expect("minimum intrinsic height of padding must be finite")
        })
    }

    fn measure(&self, constraints: Constraints) -> Size {
        let inner_constraints = constraints.deflate(&self.padding);

        let child_size = self.child.measure(inner_constraints);

        constraints
            .constrain(Size::new(self.padding.horizontal(), self.padding.vertical()) + child_size)
    }

    fn layout(&mut self, constraints: Constraints) {
        let inner_constraints = constraints.deflate(&self.padding);

        self.child.layout(inner_constraints);

        self.child_offset = Offset::new(self.padding.left, self.padding.top);

        self.size = constraints.constrain(
            Size::new(self.padding.horizontal(), self.padding.vertical()) + self.child.size(),
        )
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let inner_constraints = constraints.deflate(&self.padding);

        self.child
            .measure_baseline(inner_constraints, baseline)
            .map(|baseline| {
                PositiveFinite::try_from(baseline + self.padding.top())
                    .expect("baseline of padding must be finite")
            })
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline).map(|distance| {
            PositiveFinite::try_from(distance + self.child_offset.y)
                .expect("distance to baseline of padding was not a positive finite number")
        })
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        if !self.size().contains(position) {
            return false;
        }

        result.with_offset(self.child_offset, position, |result, transformed| {
            self.child.hit_test(result, transformed)
        })
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        canvas.with_offset(Offset::new(self.padding.left, self.padding.top), |canvas| {
            self.child.draw(canvas);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::mpsc};

    use agui_core::edge_insets::EdgeInsets;

    use super::*;
    use crate::sized_box::SizedBox;

    #[test]
    fn adds_correct_padding() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let padding = Padding::new(EdgeInsets::all(10.0)).child(());
        let mut render_object = Element::new(&padding, &mut update_ctx)
            .as_ref(&padding)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(render_object.size(), Size::new(20.0, 20.0));

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::shrink());
        let mut render_object = Element::new(&padding, &mut update_ctx)
            .as_ref(&padding)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(render_object.size(), Size::new(100.0, 100.0));

        let padding = Padding::new(EdgeInsets::all(50.0)).child(SizedBox::expand());
        let mut render_object = Element::new(&padding, &mut update_ctx)
            .as_ref(&padding)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(render_object.size(), Size::new(128.0, 128.0));
    }
}
