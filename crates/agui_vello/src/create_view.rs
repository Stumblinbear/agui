use agui_core::{
    element::{
        lifecycle::ElementLifecycle, render::ElementRender, view::ElementView, Element,
        ElementBuilder, ElementComparison, ElementMountContext, RenderObjectCreateContext,
        RenderObjectUpdateContext,
    },
    engine::rendering::view::View,
    render::object::{
        RenderObject, RenderObjectImpl, RenderObjectIntrinsicSizeContext, RenderObjectLayoutContext,
    },
    unit::{Constraints, IntrinsicDimension, Size},
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::{
    renderer::VelloRenderer,
    view::{VelloView, VelloViewHandle},
};

#[derive(WidgetProps)]
pub struct CreateVelloView<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    pub renderer: VelloRenderer,

    pub builder: BuilderFn,
}

impl<BuilderFn> IntoWidget for CreateVelloView<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl<BuilderFn> ElementBuilder for CreateVelloView<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    type Element = VelloViewElement<BuilderFn>;

    fn create_element(self: std::rc::Rc<Self>) -> Element
    where
        Self: Sized,
    {
        Element::new_view(VelloViewElement::new(
            self.renderer.clone(),
            self.builder.clone(),
        ))
    }
}

pub struct VelloViewElement<BuilderFn> {
    renderer: VelloRenderer,
    builder: BuilderFn,

    view: Option<VelloView>,
    view_handle: Option<VelloViewHandle>,
}

impl<BuilderFn> VelloViewElement<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    pub fn new(renderer: VelloRenderer, builder: BuilderFn) -> Self {
        Self {
            renderer,
            builder,

            view: None,
            view_handle: None,
        }
    }
}

impl<BuilderFn> ElementLifecycle for VelloViewElement<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    fn mount(&mut self, _: &mut ElementMountContext) {
        let (view, view_handle) = self.renderer.new_view();

        self.view = Some(view);
        self.view_handle = Some(view_handle);
    }

    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        if let Some(vello_binding) = new_widget.downcast::<CreateVelloView<BuilderFn>>() {
            if vello_binding.renderer == self.renderer {
                ElementComparison::Changed
            } else {
                // TODO: currently, create_view would not get called again if we return Changed here
                // so we have to recreate the element entirely. `is_valid_render_object` should actually
                // be called to re-use render objects in order to do that.
                ElementComparison::Invalid
            }
        } else {
            ElementComparison::Invalid
        }
    }
}

impl<BuilderFn> ElementRender for VelloViewElement<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    fn children(&self) -> Vec<Widget> {
        let view_handle = self.view_handle.as_ref().expect("view handle missing");

        vec![(self.builder)(view_handle)]
    }

    fn create_render_object(&self, _: &mut RenderObjectCreateContext) -> RenderObject {
        RenderObject::new(RenderVelloView)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        render_object.is::<RenderVelloView>()
    }

    fn update_render_object(&self, _: &mut RenderObjectUpdateContext, _: &mut RenderObject) {}
}

impl<BuilderFn> ElementView for VelloViewElement<BuilderFn>
where
    BuilderFn: Fn(&VelloViewHandle) -> Widget + Clone + 'static,
{
    fn create_view(&mut self) -> Box<dyn View + Send> {
        Box::new(self.view.take().expect("view has already been created"))
    }
}

struct RenderVelloView;

impl RenderObjectImpl for RenderVelloView {
    fn intrinsic_size(
        &self,
        ctx: &mut RenderObjectIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        // Vello views always take the size of the child
        ctx.iter_children()
            .next()
            .unwrap()
            .compute_intrinsic_size(dimension, cross_extent)
    }

    fn layout(&self, ctx: &mut RenderObjectLayoutContext, constraints: Constraints) -> Size {
        // Vello views always take the size of the child
        ctx.iter_children_mut()
            .next()
            .unwrap()
            .compute_layout(constraints)
    }
}
