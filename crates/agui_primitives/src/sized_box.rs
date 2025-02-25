use std::marker::PhantomData;

use typed_floats::{as_const, Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{MessageCtx, UpdateCtx},
    element::Element,
    hit_test::HitTestResult,
    offset::Offset,
    render_object::{
        Bounded, InheritedBound, LayoutBoundMarker, RenderObject, ResolveLayoutMarker,
        ResolveLayoutMarkerOr, Unbounded,
    },
    renderer::Canvas,
    size::Size,
    text_baseline::TextBaseline,
    view::View,
};

pub struct SizedBox<Constraints, Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: Child,

    _phantom: PhantomData<Constraints>,
}

impl Default for SizedBox<(InheritedBound, InheritedBound), ()> {
    fn default() -> Self {
        SizedBox {
            width: None,
            height: None,

            child: (),

            _phantom: PhantomData,
        }
    }
}

impl SizedBox<(InheritedBound, InheritedBound), ()> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SizedBox<(Bounded, Bounded), ()> {
    pub fn shrink() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, 0.0)),
            height: Some(as_const!(Positive, f32, 0.0)),

            child: (),

            _phantom: PhantomData,
        }
    }
}

impl SizedBox<(Unbounded, Unbounded), ()> {
    pub fn expand() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: (),

            _phantom: PhantomData,
        }
    }
}

impl<Height> SizedBox<(InheritedBound, Height), ()> {
    pub fn width<T>(self, width: T) -> SizedBox<(Bounded, Height), ()>
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        SizedBox {
            width: Some(
                PositiveFinite::try_from(width)
                    .expect("invalid width given to SizedBox")
                    .into(),
            ),
            height: self.height,

            child: self.child,

            _phantom: PhantomData,
        }
    }

    pub fn expand_width(self) -> SizedBox<(Unbounded, Height), ()> {
        SizedBox {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: self.height,

            child: self.child,

            _phantom: PhantomData,
        }
    }
}

impl<Width> SizedBox<(Width, InheritedBound), ()> {
    pub fn height<T>(self, height: T) -> SizedBox<(Width, Bounded), ()>
    where
        PositiveFinite<f32>: TryFrom<T>,
        <PositiveFinite<f32> as TryFrom<T>>::Error: std::fmt::Debug,
    {
        SizedBox {
            width: self.width,
            height: Some(
                PositiveFinite::try_from(height)
                    .expect("invalid height given to SizedBox")
                    .into(),
            ),

            child: self.child,

            _phantom: PhantomData,
        }
    }

    pub fn expand_height(self) -> SizedBox<(Width, Unbounded), ()> {
        SizedBox {
            width: self.width,
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: self.child,

            _phantom: PhantomData,
        }
    }
}

impl<AdditionalConstraints> SizedBox<AdditionalConstraints, ()> {
    pub fn child<Child>(self, child: Child) -> SizedBox<AdditionalConstraints, Child> {
        SizedBox {
            width: self.width,
            height: self.height,

            child,

            _phantom: PhantomData,
        }
    }
}

impl From<Size> for SizedBox<(Bounded, Bounded), ()> {
    fn from(size: Size) -> Self {
        Self {
            width: Some(
                PositiveFinite::<f32>::try_from(size.width)
                    .expect("width must be a positive finite number")
                    .into(),
            ),
            height: Some(
                PositiveFinite::<f32>::try_from(size.height)
                    .expect("height must be a positive finite number")
                    .into(),
            ),

            child: (),

            _phantom: PhantomData,
        }
    }
}

impl<Width, Height, Child> View for SizedBox<(Width, Height), Child>
where
    Child: View,
    ResolveLayoutMarkerOr<Width, <Child::Render as RenderObject>::Width>: ResolveLayoutMarker,
    ResolveLayoutMarkerOr<Height, <Child::Render as RenderObject>::Height>: ResolveLayoutMarker,
{
    type Render = RenderSizedBox<
        <ResolveLayoutMarkerOr<Width, <Child::Render as RenderObject>::Width> as ResolveLayoutMarker>::Value,
        <ResolveLayoutMarkerOr<Height, <Child::Render as RenderObject>::Height> as ResolveLayoutMarker>::Value,
        Child::Render,
    >;

    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child_mut(0, &old.child).update(&self.child, ctx);
    }

    fn message(&self, element: &mut Element, ctx: MessageCtx) {
        match ctx.routing_id() {
            Some(0) => element.child_mut(0, &self.child).message(ctx),
            _ => unreachable!(),
        }
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        RenderSizedBox {
            width: self.width,
            height: self.height,

            child: element.child(0, &self.child).create_render_object(),

            _phantom: PhantomData,
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        // TODO(trevin): mark it for re-layout if these have changed
        render_object.width = self.width;
        render_object.height = self.height;

        element
            .child(0, &self.child)
            .update_render_object(&mut render_object.child);
    }
}
pub struct RenderSizedBox<Width, Height, Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: Child,

    _phantom: PhantomData<(Width, Height)>,
}

impl<Width, Height, Child> RenderSizedBox<Width, Height, Child> {
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

impl<Width, Height, Child> RenderObject for RenderSizedBox<Width, Height, Child>
where
    Child: RenderObject,
    Width: LayoutBoundMarker,
    Height: LayoutBoundMarker,
{
    type Width = Width;
    type Height = Height;

    type WidthIntrinsic = Child::WidthIntrinsic;
    type HeightIntrinsic = Child::HeightIntrinsic;

    fn mount(&mut self, ctx: &mut UpdateCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        self.child.unmount(ctx);
    }

    fn size(&self) -> Size {
        self.child.size()
    }

    fn min_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.width
            .and_then(|width| PositiveFinite::try_from(width).ok())
            .or_else(|| self.child.min_intrinsic_width(height))
    }

    fn max_intrinsic_width(&self, height: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.width
            .and_then(|width| PositiveFinite::try_from(width).ok())
            .or_else(|| self.child.max_intrinsic_width(height))
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.height
            .and_then(|height| PositiveFinite::try_from(height).ok())
            .or_else(|| self.child.min_intrinsic_height(width))
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.height
            .and_then(|height| PositiveFinite::try_from(height).ok())
            .or_else(|| self.child.max_intrinsic_height(width))
    }

    fn measure(&self, constraints: Constraints) -> Size {
        self.child
            .measure(self.additional_constraints().enforce(constraints))
    }

    fn layout(&mut self, constraints: Constraints) {
        self.child
            .layout(self.additional_constraints().enforce(constraints));
    }

    fn measure_baseline(
        &self,
        constraints: Constraints,
        baseline: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        self.child
            .measure_baseline(self.additional_constraints().enforce(constraints), baseline)
    }

    fn distance_to_baseline(&mut self, baseline: TextBaseline) -> Option<PositiveFinite<f32>> {
        self.child.distance_to_baseline(baseline)
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> bool {
        if !self.size().contains(position) {
            return false;
        }

        self.child.hit_test(result, position)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        self.child.draw(canvas);
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::mpsc};

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn results_in_correct_sizing() {
        let (tx, _) = mpsc::channel();
        let mut path = VecDeque::new();

        let sized_box = SizedBox::new().width(16).height(48);
        let mut render_object = Element::new(&sized_box, &mut UpdateCtx::new(&tx, &mut path))
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(16, 48),
            "should use the given sizes"
        );

        let sized_box = SizedBox::new().width(0).height(16);
        let mut render_object = Element::new(&sized_box, &mut UpdateCtx::new(&tx, &mut path))
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(16, 128, 32, 128));
        assert_eq!(
            render_object.size(),
            Size::new(16, 32),
            "should ignore the given sizes and use the smallest size allowed by the constraints"
        );

        let sized_box = SizedBox::shrink();
        let mut render_object = Element::new(&sized_box, &mut UpdateCtx::new(&tx, &mut path))
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(0, 0),
            "should shrink to the smallest size possible"
        );

        let sized_box = SizedBox::shrink();
        let mut render_object = Element::new(&sized_box, &mut UpdateCtx::new(&tx, &mut path))
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(10, 128, 20, 128));
        assert_eq!(
            render_object.size(),
            Size::new(10, 20),
            "should shrink to the smallest size possible within the constraints"
        );

        let sized_box = SizedBox::expand();
        let mut render_object = Element::new(&sized_box, &mut UpdateCtx::new(&tx, &mut path))
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(128, 128),
            "should expand to the largest size possible within the constraints"
        );
    }
}
