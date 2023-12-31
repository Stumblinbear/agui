use std::{cell::RefCell, rc::Rc, sync::Arc};

use parking_lot::Mutex;

use crate::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementBuildContext, ElementBuilder,
        ElementMountContext, ElementType, ElementUnmountContext, ElementUpdate,
        RenderObjectBuildContext, RenderObjectUpdateContext,
    },
    render::{MockRenderObjectImpl, RenderObject},
    widget::{IntoWidget, Widget},
};

#[allow(clippy::disallowed_types)]
#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait RenderElement {
    fn mount<'ctx>(&mut self, ctx: &mut ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: &mut ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;

    fn children(&self) -> Vec<Widget>;

    fn create_render_object<'ctx>(&mut self, ctx: &mut ElementBuildContext<'ctx>) -> RenderObject;

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
    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::Render(Box::new(MockElement::new(self)))
    }
}

struct MockElement {
    widget: Rc<MockRenderWidget>,
}

impl MockElement {
    pub fn new(widget: Rc<MockRenderWidget>) -> Self {
        Self { widget }
    }
}

impl ElementWidget for MockElement {
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementRender for MockElement {
    fn children(&self) -> Vec<Widget> {
        self.widget.mock.borrow().children()
    }

    fn create_render_object(&mut self, ctx: &mut RenderObjectBuildContext) -> RenderObject {
        self.widget.mock.borrow_mut().create_render_object(ctx)
    }

    fn is_valid_render_object(&self, render_object: &RenderObject) -> bool {
        self.widget
            .mock
            .borrow()
            .is_valid_render_object(render_object)
    }

    fn update_render_object(
        &mut self,
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
