use std::{cell::RefCell, rc::Rc, sync::Arc};

use parking_lot::Mutex;

use crate::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementBuilder, ElementMountContext,
        ElementType, ElementUnmountContext, ElementUpdate,
    },
    render::{MockRenderObjectImpl, RenderObject},
    widget::{IntoWidget, Widget},
};

#[allow(clippy::disallowed_types)]
#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait RenderElement {
    fn widget_name(&self) -> &'static str;

    fn mount<'ctx>(&mut self, ctx: ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;

    fn children(&self) -> Vec<Widget>;

    fn create_render_object(&self) -> RenderObject;

    fn update_render_object(&self, render_object: &mut RenderObject);
}

#[derive(Default)]
pub struct MockRenderWidget {
    pub mock: Rc<RefCell<MockRenderElement>>,
}

impl MockRenderWidget {
    pub fn new(name: &'static str) -> Self {
        let mut mock = MockRenderElement::default();

        mock.expect_widget_name().returning(move || name);

        Self {
            mock: Rc::new(RefCell::new(mock)),
        }
    }
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
    fn widget_name(&self) -> &'static str {
        self.widget.mock.borrow().widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementRender for MockElement {
    fn children(&self) -> Vec<Widget> {
        self.widget.mock.borrow().children()
    }

    fn create_render_object(&self) -> RenderObject {
        self.widget.mock.borrow().create_render_object()
    }

    fn update_render_object(&self, render_object: &mut RenderObject) {
        self.widget
            .mock
            .borrow()
            .update_render_object(render_object)
    }
}

#[derive(Default)]
pub struct MockRenderObject {
    pub mock: Arc<Mutex<MockRenderObjectImpl>>,
}

impl MockRenderObject {
    pub fn new(name: &'static str) -> Self {
        let mut mock = MockRenderObjectImpl::default();

        mock.expect_render_object_name().returning(move || name);

        Self {
            mock: Arc::new(Mutex::new(mock)),
        }
    }
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
