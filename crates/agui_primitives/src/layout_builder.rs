use std::{cell::RefCell, marker::PhantomData, rc::Rc};

use typed_floats::{Positive, PositiveFinite};

use agui_core::{
    constraints::Constraints,
    context::{Dispatch, UpdateCtx},
    element::Element,
    hit_test::{HitTest, HitTestResult},
    offset::Offset,
    render_object::{
        RenderNode, RenderObject,
        box_layout::{BoxLayout, RenderBox},
    },
    renderer::Canvas,
    routing_id::RoutingId,
    size::Size,
    text_baseline::TextBaseline,
    view::View,
};

pub struct LayoutBuilder<F, Child> {
    builder: Rc<F>,

    _phantom: PhantomData<Child>,
}

impl<F, Child> LayoutBuilder<F, Child>
where
    F: Fn(Constraints) -> Child,
{
    pub fn new(builder: F) -> Self {
        Self {
            builder: Rc::new(builder),

            _phantom: PhantomData,
        }
    }
}

pub struct LayoutBuilderState<Child>
where
    Child: View,
{
    child_view: Rc<RefCell<Option<(Element, Child)>>>,

    builder: Rc<dyn Fn(Constraints) -> Child::Render>,
}

impl<F, Child> View for LayoutBuilder<F, Child>
where
    F: Fn(Constraints) -> Child + 'static,
    Child: View + 'static,
    Child::Render: RenderBox,
{
    type Render = RenderLayoutBuilder<Child::Render>;

    type State = LayoutBuilderState<Child>;

    fn mount(&self, ctx: &mut UpdateCtx) -> (Vec<Element>, Self::State) {
        let child_view = Rc::<RefCell<Option<(Element, Child)>>>::default();

        let builder = {
            let builder = Rc::clone(&self.builder);
            let child_view = Rc::clone(&child_view);

            let driver = Rc::clone(ctx.driver());
            let event_tx = ctx.event_tx().clone();
            let routing_path = ctx.routing_path();
            let provide_scope = ctx.provide_scope().clone();

            Rc::new(move |constraints| {
                let mut routing_path = routing_path.to_vec();

                let child = (builder)(constraints);

                // TODO(trevin): try to use the element of the previously created child
                let mut element = Element::new(
                    &child,
                    &mut UpdateCtx::new(&driver, &event_tx, &mut routing_path, &provide_scope),
                );

                let child_render = element.as_mut(&child).create_render_object();

                child_view.replace(Some((element, child)));

                child_render
            })
        };

        (
            vec![],
            LayoutBuilderState {
                child_view,

                builder,
            },
        )
    }

    fn update(&self, element: &mut Element, old: &Self, ctx: &mut UpdateCtx) {
        if !Rc::ptr_eq(&self.builder, &old.builder) {
            let state = element.state.downcast_mut::<Self>();

            state.child_view.replace(None);

            state.builder = {
                let builder = Rc::clone(&self.builder);
                let child_view = Rc::clone(&state.child_view);

                let driver = Rc::clone(ctx.driver());
                let event_tx = ctx.event_tx().clone();
                let routing_path = ctx.routing_path();
                let provide_scope = ctx.provide_scope().clone();

                Rc::new(move |constraints| {
                    let child = (builder)(constraints);

                    let mut routing_path = routing_path.to_vec();

                    // TODO(trevin): try to use the element of the previously created child
                    let mut element = Element::new(
                        &child,
                        &mut UpdateCtx::new(&driver, &event_tx, &mut routing_path, &provide_scope),
                    );

                    let child_render = element.as_mut(&child).create_render_object();

                    child_view.replace(Some((element, child)));

                    child_render
                })
            };
        }
    }

    fn dispatch(&self, element: &mut Element, path: &[RoutingId], action: Dispatch) {
        let mut child_view = element.state.downcast_ref::<Self>().child_view.borrow_mut();

        let Some((child_element, child_view)) = child_view.as_mut() else {
            panic!("child was dispatched to before being laid out");
        };

        child_element.as_mut(child_view).dispatch(path, action)
    }

    fn create_render_object(&self, element: &Element) -> Self::Render {
        RenderLayoutBuilder {
            builder: Rc::clone(&element.state.downcast_ref::<Self>().builder),

            old_constraints: Constraints::default(),

            child_render: None,
        }
    }

    fn update_render_object(&self, element: &Element, render_object: &mut Self::Render) {
        let state = element.state.downcast_ref::<Self>();

        if !Rc::ptr_eq(&state.builder, &render_object.builder) {
            // TODO(trevin): mark for re-layout
            render_object.builder = Rc::clone(&state.builder);

            render_object.child_render.take();
        }

        let child_view = state.child_view.borrow();

        if let Some((element, child)) = child_view.as_ref() {
            if let Some(child_render) = &mut render_object.child_render {
                element
                    .child(0, child)
                    .update_render_object(&mut child_render.object);
            }
        } else if render_object.child_render.is_some() {
            // TODO(trevin): mark for re-layout
            render_object.child_render = None;
        }
    }
}

pub struct RenderLayoutBuilder<Child> {
    builder: Rc<dyn Fn(Constraints) -> Child>,

    old_constraints: Constraints,

    child_render: Option<RenderNode<Child, Option<Size>>>,
}

impl<Child> RenderObject for RenderLayoutBuilder<Child>
where
    Child: RenderBox,
{
    fn mount(&mut self, _: &mut UpdateCtx) {}

    fn unmount(&mut self, ctx: &mut UpdateCtx) {
        if let Some(mut child_render) = self.child_render.take() {
            child_render.unmount(ctx);
        }
    }

    fn hit_test(&self, result: &mut HitTestResult, offset: Offset) -> HitTest {
        if let Some(child_render) = self.child_render.as_ref() {
            child_render.hit_test(result, offset)
        } else {
            HitTest::Pass
        }
    }

    fn paint(&mut self, canvas: &mut Canvas) {
        if let Some(child_render) = self.child_render.as_mut() {
            child_render.paint(canvas);
        }
    }
}

impl<Child> BoxLayout for RenderLayoutBuilder<Child>
where
    Child: RenderBox,
{
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, _: Constraints) -> Size {
        Size::ZERO
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        if self.child_render.is_none() || self.old_constraints != constraints {
            self.old_constraints = constraints;

            let child = (self.builder)(constraints);

            self.child_render.replace(RenderNode::new(child));
        }

        if let Some(child_render) = self.child_render.as_mut() {
            let size = child_render.layout_and_get_size(constraints);
            child_render.parent_data = Some(size);
            size
        } else {
            Size::ZERO
        }
    }

    fn measure_baseline(&self, _: Constraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use agui_core::{test_harness::TestHarness, view::AsAnyView};

    use super::*;
    use crate::sized_box::SizedBox;

    #[test]
    fn calls_closure_during_layout() {
        let build_count = Rc::new(RefCell::new(0));

        let layout_builder = LayoutBuilder::new({
            let build_count = Rc::clone(&build_count);

            move |constraints| {
                *build_count.borrow_mut() += 1;

                if constraints.max_width() > 100.0 {
                    SizedBox::expand().into_boxed_view()
                } else {
                    SizedBox::shrink().into_boxed_view()
                }
            }
        });

        let mut render_object = TestHarness::mount(&layout_builder)
            .root
            .as_ref(&layout_builder)
            .create_render_object();
        render_object.layout(Constraints::new(0, 50, 0, 50));
        assert_eq!(*build_count.borrow(), 1);
        assert_eq!(
            render_object.child_render.as_ref().unwrap().parent_data,
            Some(Size::new(0.0, 0.0))
        );

        render_object.layout(Constraints::new(0, 150, 0, 150));
        assert_eq!(*build_count.borrow(), 2);
        assert_eq!(
            render_object.child_render.as_ref().unwrap().parent_data,
            Some(Size::new(150.0, 150.0))
        );
    }
}
