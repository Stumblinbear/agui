use typed_floats::{as_const, Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, MessageCtx, UpdateCtx},
    element::Element,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::{
        box_layout::{BoxLayout, RenderBox},
        AsAnyRenderObject, RenderObject,
    },
    renderer::Canvas,
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    view::View,
};

pub struct SizedBox<Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: Child,
}

impl Default for SizedBox<()> {
    fn default() -> Self {
        SizedBox {
            width: None,
            height: None,

            child: (),
        }
    }
}

impl SizedBox<()> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SizedBox<()> {
    pub fn shrink() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, 0.0)),
            height: Some(as_const!(Positive, f32, 0.0)),

            child: (),
        }
    }
}

impl SizedBox<()> {
    pub fn expand() -> Self {
        Self {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: (),
        }
    }
}

impl SizedBox<()> {
    pub fn width<T>(self, width: T) -> SizedBox<()>
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
        }
    }

    pub fn expand_width(self) -> SizedBox<()> {
        SizedBox {
            width: Some(as_const!(Positive, f32, f32::INFINITY)),
            height: self.height,

            child: self.child,
        }
    }
}

impl SizedBox<()> {
    pub fn height<T>(self, height: T) -> SizedBox<()>
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
        }
    }

    pub fn expand_height(self) -> SizedBox<()> {
        SizedBox {
            width: self.width,
            height: Some(as_const!(Positive, f32, f32::INFINITY)),

            child: self.child,
        }
    }
}

impl SizedBox<()> {
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
        }
    }
}

impl<Child> View for SizedBox<Child>
where
    Child: View,
    Child::Render: RenderBox,
{
    type Render = RenderSizedBox<Child::Render>;

    type State = ();

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        (vec![Element::new(&self.child, ctx)], ())
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        element.child_mut(0, &old.child).update(&self.child, ctx);
    }

    fn rebuild(&self, element: &mut Element, ctx: &mut UpdateCtx) {
        element.child_mut(0, &self.child).rebuild(ctx);
    }

    fn message(&self, _: &mut Element, _: &mut MessageCtx) {}

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        element.child_mut(0, &self.child).dispatch(path, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        RenderSizedBox {
            width: self.width,
            height: self.height,

            child: element.child(0, &self.child).create_render_object(),
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
pub struct RenderSizedBox<Child> {
    width: Option<Positive<f32>>,
    height: Option<Positive<f32>>,

    child: Child,
}

impl<Child> RenderSizedBox<Child> {
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

impl<Child> RenderObject for RenderSizedBox<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, ctx: &mut UpdateCtx) {
        self.child.mount(ctx);
    }

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        self.child.unmount(ctx);
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        if !self.size().contains(position) {
            return HitTest::Pass;
        }

        self.child.hit_test(result, position)
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        self.child.draw(canvas);
    }
}

impl<Child> BoxLayout for RenderSizedBox<Child>
where
    Child: RenderBox,
{
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
}

impl<Child> AsAnyRenderObject for RenderSizedBox<Child>
where
    Self: RenderBox,
{
    type Output = dyn agui_core::render_object::box_layout::AnyRenderBox;

    fn as_dyn_render_object(&self) -> &dyn agui_core::render_object::AnyRenderObject {
        self
    }

    fn into_boxed_render_object(self) -> Box<Self::Output> {
        Box::new(self)
    }
}

#[cfg(test)]
mod tests {
    use agui_core::test_harness::TestHarness;

    use crate::sized_box::SizedBox;

    use super::*;

    #[test]
    fn results_in_correct_sizing() {
        let sized_box = SizedBox::new().width(16).height(48);
        let mut render_object = TestHarness::mount(&sized_box)
            .root
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(16, 48),
            "should use the given sizes"
        );

        let sized_box = SizedBox::new().width(0).height(16);
        let mut render_object = TestHarness::mount(&sized_box)
            .root
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(16, 128, 32, 128));
        assert_eq!(
            render_object.size(),
            Size::new(16, 32),
            "should ignore the given sizes and use the smallest size allowed by the constraints"
        );

        let sized_box = SizedBox::shrink();
        let mut render_object = TestHarness::mount(&sized_box)
            .root
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(0, 128, 0, 128));
        assert_eq!(
            render_object.size(),
            Size::new(0, 0),
            "should shrink to the smallest size possible"
        );

        let sized_box = SizedBox::shrink();
        let mut render_object = TestHarness::mount(&sized_box)
            .root
            .as_ref(&sized_box)
            .create_render_object();
        render_object.layout(Constraints::new(10, 128, 20, 128));
        assert_eq!(
            render_object.size(),
            Size::new(10, 20),
            "should shrink to the smallest size possible within the constraints"
        );

        let sized_box = SizedBox::expand();
        let mut render_object = TestHarness::mount(&sized_box)
            .root
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
