use std::{cell::RefCell, rc::Rc};

use crate::{
    element::{
        inherited::ElementInherited, widget::ElementWidget, ElementBuilder, ElementComparison,
        ElementMountContext, ElementType, ElementUnmountContext,
    },
    widget::{IntoWidget, Widget},
};

#[allow(clippy::disallowed_types)]
#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait InheritedElement {
    fn mount<'ctx>(&mut self, ctx: &mut ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: &mut ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementComparison;

    fn child(&self) -> Widget;

    fn needs_notify(&mut self) -> bool;
}

#[derive(Default)]
pub struct MockInheritedWidget {
    pub mock: Rc<RefCell<MockInheritedElement>>,
}

impl MockInheritedWidget {
    pub fn new(mock: MockInheritedElement) -> Self {
        Self {
            mock: Rc::new(RefCell::new(mock)),
        }
    }
}

impl IntoWidget for MockInheritedWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockInheritedWidget {
    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::new_inherited(MockElement::new(self))
    }
}

struct MockElement {
    widget: Rc<MockInheritedWidget>,
}

impl MockElement {
    pub fn new(widget: Rc<MockInheritedWidget>) -> Self {
        Self { widget }
    }
}

impl ElementWidget for MockElement {
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementInherited for MockElement {
    fn child(&self) -> Widget {
        self.widget.mock.borrow().child()
    }

    fn needs_notify(&mut self) -> bool {
        self.widget.mock.borrow_mut().needs_notify()
    }
}
