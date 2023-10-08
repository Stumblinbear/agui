use std::{cell::RefCell, rc::Rc};

use crate::{
    element::{
        proxy::ElementProxy, widget::ElementWidget, ElementBuilder, ElementMountContext,
        ElementType, ElementUnmountContext, ElementUpdate,
    },
    widget::{IntoWidget, Widget},
};

#[allow(clippy::disallowed_types)]
#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait ProxyElement {
    fn widget_name(&self) -> &'static str;

    fn mount<'ctx>(&mut self, ctx: ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;

    fn get_child(&self) -> Widget;
}

#[derive(Default)]
pub struct MockProxyWidget {
    pub mock: Rc<RefCell<MockProxyElement>>,
}

impl MockProxyWidget {
    pub fn new(name: &'static str) -> Self {
        let mut mock = MockProxyElement::default();

        mock.expect_widget_name().returning(move || name);

        Self {
            mock: Rc::new(RefCell::new(mock)),
        }
    }
}

impl IntoWidget for MockProxyWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockProxyWidget {
    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::Proxy(Box::new(MockElement::new(self)))
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
    fn widget_name(&self) -> &'static str {
        self.widget.mock.borrow().widget_name()
    }

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementProxy for MockElement {
    fn get_child(&self) -> Widget {
        self.widget.mock.borrow().get_child()
    }
}
