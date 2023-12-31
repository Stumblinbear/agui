use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    callback::CallbackId,
    element::{
        build::ElementBuild, widget::ElementWidget, ElementBuildContext, ElementBuilder,
        ElementCallbackContext, ElementMountContext, ElementType, ElementUnmountContext,
        ElementUpdate,
    },
    widget::{IntoWidget, Widget},
};

#[allow(clippy::disallowed_types)]
#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait ProxyElement {
    fn mount<'ctx>(&mut self, ctx: &mut ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: &mut ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;

    fn child(&self) -> Widget;
}

#[derive(Default)]
pub struct MockProxyWidget {
    pub mock: Rc<RefCell<MockProxyElement>>,
}

impl IntoWidget for MockProxyWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockProxyWidget {
    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::Widget(Box::new(MockElement::new(self)))
    }
}

struct MockElement {
    widget: Rc<MockProxyWidget>,
}

impl MockElement {
    pub fn new(widget: Rc<MockProxyWidget>) -> Self {
        Self { widget }
    }
}

impl ElementWidget for MockElement {
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementBuild for MockElement {
    fn build(&mut self, _: &mut ElementBuildContext) -> Widget {
        self.widget.mock.borrow().child()
    }

    fn call(&mut self, _: &mut ElementCallbackContext, _: CallbackId, _: Box<dyn Any>) -> bool {
        false
    }
}
