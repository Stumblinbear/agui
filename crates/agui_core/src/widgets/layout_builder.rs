use std::marker::PhantomData;

use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    view::{NoIntrinsic, View, ViewLayoutMarker},
    view_id::ViewId,
};

pub struct LayoutBuilder<F, Child> {
    builder: F,

    _phantom: PhantomData<Child>,
}

impl<F, Child> LayoutBuilder<F, Child>
where
    F: Fn(Constraints) -> Child,
{
    pub fn new(builder: F) -> Self {
        Self {
            builder,

            _phantom: PhantomData,
        }
    }
}

pub struct LayoutBuilderState<Child> {
    child: Option<Child>,
}

impl<F, Child> ViewLayoutMarker for LayoutBuilder<F, Child>
where
    Child: ViewLayoutMarker,
{
    type Width = Child::Width;
    type Height = Child::Height;

    type WidthIntrinsic = NoIntrinsic;
    type HeightIntrinsic = NoIntrinsic;
}

impl<F, Child> View for LayoutBuilder<F, Child>
where
    F: Fn(Constraints) -> Child,
    Child: View + 'static,
{
    type State = LayoutBuilderState<Child>;

    fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![], LayoutBuilderState { child: None })
    }

    fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

    fn message(&self, element: &mut Element, ctx: MessageCtx) {
        let Some(0) = ctx.routing_id() else {
            unreachable!();
        };

        let Some(child) = element.state.downcast_ref::<Self>().child.as_ref() else {
            unreachable!("child was sent a message before being laid out");
        };

        element.children[0]
            .as_mut(ViewId::new(0), child)
            .message(ctx);
    }

    fn min_intrinsic_width(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _: &Element, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, _: &Element, _: Constraints) -> Size {
        unimplemented!()
    }

    fn layout(&self, _: &mut Element, _: Constraints) -> Size {
        unimplemented!()
    }

    fn measure_baseline(
        &self,
        _: &Element,
        _: Constraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(
        &self,
        _: &mut Element,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &Element, _: &mut HitTestResult, _: Offset) -> bool {
        unimplemented!()
    }

    fn draw(&self, _: &mut Element, _: &mut Canvas) {
        unimplemented!()
    }
}
