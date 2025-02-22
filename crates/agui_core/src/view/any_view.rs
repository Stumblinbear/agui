use std::{any::Any, rc::Rc, sync::Arc};

use typed_floats::{Positive, PositiveFinite};

use crate::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::{Element, ElementState},
    hit_test::HitTestResult,
    offset::Offset,
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    view::{LayoutBoundMarker, LayoutIntrinsicMarker, View, ViewLayoutMarker},
};

pub trait AnyView: ViewLayoutMarker {
    fn as_any(&self) -> &dyn Any;

    fn view_name(&self) -> &str;

    fn dyn_mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState);

    fn dyn_update(
        &self,
        element: &mut Element,
        old: &dyn AnyView<
            Width = Self::Width,
            Height = Self::Height,
            WidthIntrinsic = Self::WidthIntrinsic,
            HeightIntrinsic = Self::HeightIntrinsic,
        >,
        ctx: &mut UpdateCtx,
    );

    fn dyn_message(&self, element: &mut Element, ctx: MessageCtx);

    fn dyn_min_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_min_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_max_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_measure(&self, element: &Element, constraints: Constraints) -> Size;

    fn dyn_layout(&self, element: &mut Element, constraints: Constraints) -> Size;

    fn dyn_measure_baseline(
        &self,
        element: &Element,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_distance_to_baseline(
        &self,
        element: &mut Element,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>>;

    fn dyn_hit_test(&self, element: &Element, result: &mut HitTestResult, position: Offset)
        -> bool;

    fn dyn_draw(&self, element: &mut Element, canvas: &mut Canvas);
}

impl<T, Width, Height> AnyView for T
where
    T: Any,
    T: View<Width = Width, Height = Height>,
    Width: LayoutBoundMarker,
    Height: LayoutBoundMarker,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn view_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, ElementState) {
        let (children, state) = self.mount(ctx);

        (children, ElementState::new(state))
    }

    fn dyn_update(
        &self,
        element: &mut Element,
        old: &dyn AnyView<
            Width = Self::Width,
            Height = Self::Height,
            WidthIntrinsic = Self::WidthIntrinsic,
            HeightIntrinsic = Self::HeightIntrinsic,
        >,
        ctx: &mut UpdateCtx,
    ) {
        let old = old
            .as_any()
            .downcast_ref::<Self>()
            .expect("downcast failed");

        self.update(element, old, ctx);
    }

    fn dyn_message(&self, element: &mut Element, ctx: MessageCtx) {
        self.message(element, ctx);
    }

    fn dyn_min_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.min_intrinsic_width(element, height)
    }

    fn dyn_max_intrinsic_width(
        &self,
        element: &Element,
        height: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_width(element, height)
    }

    fn dyn_min_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.min_intrinsic_height(element, width)
    }

    fn dyn_max_intrinsic_height(
        &self,
        element: &Element,
        width: Positive<f32>,
    ) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_height(element, width)
    }

    fn dyn_measure(&self, element: &Element, constraints: Constraints) -> Size {
        self.measure(element, constraints)
    }

    fn dyn_layout(&self, element: &mut Element, constraints: Constraints) -> Size {
        self.layout(element, constraints)
    }

    fn dyn_measure_baseline(
        &self,
        element: &Element,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.measure_baseline(element, constraints, baseline)
    }

    fn dyn_distance_to_baseline(
        &self,
        element: &mut Element,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.distance_to_baseline(element, baseline)
    }

    fn dyn_hit_test(
        &self,
        element: &Element,
        result: &mut HitTestResult,
        position: Offset,
    ) -> bool {
        self.hit_test(element, result, position)
    }

    fn dyn_draw(&self, element: &mut Element, canvas: &mut Canvas) {
        self.draw(element, canvas);
    }
}

macros::impl_view!(
    &dyn AnyView<
        Width = Width,
        Height = Height,
        WidthIntrinsic = WidthIntrinsic,
        HeightIntrinsic = HeightIntrinsic,
    >
);

macros::impl_view!(
    Box<
        dyn AnyView<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
);

macros::impl_view!(
    Rc<
        dyn AnyView<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
);

macros::impl_view!(
    Arc<
        dyn AnyView<
            Width = Width,
            Height = Height,
            WidthIntrinsic = WidthIntrinsic,
            HeightIntrinsic = HeightIntrinsic,
        >,
    >
);

mod macros {
    // Used to implement View for the given smart pointer (e.g. Box, Rc, Arc)
    macro_rules! impl_view {
        (
            // The smart pointer type
            $ptr:ty
        ) => {
            impl<Width, Height, WidthIntrinsic, HeightIntrinsic> View for $ptr
            where
                Width: LayoutBoundMarker,
                Height: LayoutBoundMarker,
                WidthIntrinsic: LayoutIntrinsicMarker,
                HeightIntrinsic: LayoutIntrinsicMarker,
            {
                type State = ElementState;

                fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
                    (**self).dyn_mount(ctx)
                }

                fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
                    (**self).dyn_update(element, &**old, ctx);
                }

                fn message(&self, element: &mut Element, ctx: MessageCtx) {
                    (**self).dyn_message(element, ctx);
                }

                fn min_intrinsic_width(
                    &self,
                    element: &Element,
                    height: Positive<f32>,
                ) -> Option<PositiveFinite<f32>> {
                    (**self).dyn_min_intrinsic_width(element, height)
                }

                fn max_intrinsic_width(
                    &self,
                    element: &Element,
                    height: Positive<f32>,
                ) -> Option<PositiveFinite<f32>> {
                    (**self).dyn_max_intrinsic_width(element, height)
                }

                fn min_intrinsic_height(
                    &self,
                    element: &Element,
                    width: Positive<f32>,
                ) -> Option<PositiveFinite<f32>> {
                    (**self).dyn_min_intrinsic_height(element, width)
                }

                fn max_intrinsic_height(
                    &self,
                    element: &Element,
                    width: Positive<f32>,
                ) -> Option<PositiveFinite<f32>> {
                    (**self).dyn_max_intrinsic_height(element, width)
                }

                fn measure(&self, element: &Element, constraints: Constraints) -> Size {
                    (**self).dyn_measure(element, constraints)
                }

                fn layout(&self, element: &mut Element, constraints: Constraints) -> Size {
                    (**self).dyn_layout(element, constraints)
                }

                fn measure_baseline(
                    &self,
                    element: &Element,
                    constraints: Constraints,
                    baseline: TextBaseline,
                ) -> Option<PositiveFinite<f32>> {
                    (**self).dyn_measure_baseline(element, constraints, baseline)
                }

                fn distance_to_baseline(
                    &self,
                    element: &mut Element,
                    baseline: TextBaseline,
                ) -> Option<PositiveFinite<f32>> {
                    (**self).dyn_distance_to_baseline(element, baseline)
                }

                fn hit_test(
                    &self,
                    element: &Element,
                    result: &mut HitTestResult,
                    position: Offset,
                ) -> bool {
                    (**self).dyn_hit_test(element, result, position)
                }

                fn draw(&self, element: &mut Element, canvas: &mut Canvas) {
                    (**self).dyn_draw(element, canvas);
                }
            }
        };
    }

    pub(crate) use impl_view;
}

pub trait AsAnyView: View {
    fn as_dyn_view(
        &self,
    ) -> &(dyn AnyView<
        Width = Self::Width,
        Height = Self::Height,
        WidthIntrinsic = Self::WidthIntrinsic,
        HeightIntrinsic = Self::HeightIntrinsic,
    >)
    where
        Self: Sized + 'static,
    {
        self
    }

    fn into_boxed_view(
        self,
    ) -> Box<
        dyn AnyView<
            Width = Self::Width,
            Height = Self::Height,
            WidthIntrinsic = Self::WidthIntrinsic,
            HeightIntrinsic = Self::HeightIntrinsic,
        >,
    >
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl<T> AsAnyView for T where T: View {}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::mpsc};

    use crate::view::{NoIntrinsic, Unbounded};

    use super::*;

    pub struct TestView;

    impl ViewLayoutMarker for TestView {
        type Width = Unbounded;
        type Height = Unbounded;

        type WidthIntrinsic = NoIntrinsic;
        type HeightIntrinsic = NoIntrinsic;
    }

    impl View for TestView {
        type State = ();

        fn mount(&self, _: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
            (vec![], Default::default())
        }

        fn update(&self, _: &mut Element, _: &Self, _: &mut UpdateCtx) {}

        fn message(&self, _: &mut Element, _: MessageCtx) {}

        fn min_intrinsic_width(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_width(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn min_intrinsic_height(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn max_intrinsic_height(
            &self,
            _: &Element,
            _: Positive<f32>,
        ) -> Option<PositiveFinite<f32>> {
            None
        }

        fn measure(&self, _: &Element, _: Constraints) -> Size {
            Size::ZERO
        }

        fn layout(&self, _: &mut Element, _: Constraints) -> Size {
            Size::ZERO
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
            false
        }

        fn draw(&self, _: &mut Element, _: &mut Canvas) {}
    }

    #[test]
    fn mounting_dyn_views() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();
        let mut update_ctx = UpdateCtx::new(&tx, &mut path);

        let _ = Element::new(&TestView.as_dyn_view(), &mut update_ctx);

        let _ = Element::new(&TestView.into_boxed_view(), &mut update_ctx);
    }
}
