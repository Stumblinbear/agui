use std::{cell::RefCell, rc::Rc, sync::Arc};

use parking_lot::Mutex;

use crate::{
    element::{
        lifecycle::ElementLifecycle, render::ElementRender, widget::ElementWidget, ElementBuilder,
        ElementComparison, ElementMountContext, ElementType, ElementUnmountContext,
        RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    render::object::{MockRenderObjectImpl, RenderObject},
    widget::{IntoWidget, Widget},
};

#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait RenderElement {
    fn mount<'ctx>(&mut self, ctx: &mut ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: &mut ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementComparison;

    fn children(&self) -> Vec<Widget>;

    fn create_render_object<'ctx>(
        &mut self,
        ctx: &mut RenderObjectCreateContext<'ctx>,
    ) -> RenderObject;

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool;

    fn update_render_object<'ctx>(
        &mut self,
        ctx: &mut RenderObjectUpdateContext<'ctx>,
        render_object: &mut RenderObject,
    );
}

#[derive(Default)]
pub struct MockRenderWidget {
    pub mock: Rc<RefCell<MockRenderElement>>,
}

impl IntoWidget for MockRenderWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockRenderWidget {
    type Element = MockedElementRender;

    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::new_render(MockedElementRender::new(self))
    }
}

pub struct MockedElementRender {
    widget: Rc<MockRenderWidget>,
}

impl MockedElementRender {
    pub fn new(widget: Rc<MockRenderWidget>) -> Self {
        Self { widget }
    }
}

impl ElementLifecycle for MockedElementRender {
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementWidget for MockedElementRender {
    type Widget = MockRenderWidget;

    fn widget(&self) -> &Rc<Self::Widget> {
        &self.widget
    }
}

impl ElementRender for MockedElementRender {
    fn children(&self) -> Vec<Widget> {
        self.widget.mock.borrow().children()
    }

    fn create_render_object(&self, ctx: &mut RenderObjectCreateContext) -> RenderObject {
        self.widget.mock.borrow_mut().create_render_object(ctx)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        self.widget
            .mock
            .borrow()
            .is_valid_render_object(render_object)
    }

    fn update_render_object(
        &self,
        ctx: &mut RenderObjectUpdateContext,
        render_object: &mut RenderObject,
    ) {
        self.widget
            .mock
            .borrow_mut()
            .update_render_object(ctx, render_object)
    }
}

#[derive(Default)]
pub struct MockRenderObject {
    pub mock: Arc<Mutex<MockRenderObjectImpl>>,
}

impl From<MockRenderObject> for RenderObject {
    fn from(value: MockRenderObject) -> Self {
        RenderObject::new(
            Arc::into_inner(value.mock)
                .expect("cannot convert mock to render object as a reference is still held")
                .into_inner(),
        )
    }
}
