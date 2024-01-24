use std::{any::TypeId, cell::RefCell, rc::Rc};

use crate::{
    element::{
        inherited::ElementInherited, lifecycle::ElementLifecycle, widget::ElementWidget,
        ElementBuilder, ElementComparison, ElementMountContext, ElementType, ElementUnmountContext,
    },
    widget::{IntoWidget, Widget},
};

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
    type Element = MockedElementInherited;

    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::new_inherited(MockedElementInherited::new(self))
    }
}

pub struct MockedElementInherited {
    widget: Rc<MockInheritedWidget>,
}

impl MockedElementInherited {
    pub fn new(widget: Rc<MockInheritedWidget>) -> Self {
        Self { widget }
    }
}

impl ElementLifecycle for MockedElementInherited {
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementWidget for MockedElementInherited {
    type Widget = MockInheritedWidget;

    fn widget(&self) -> &Rc<Self::Widget> {
        &self.widget
    }
}

impl ElementInherited for MockedElementInherited {
    type Data = Rc<MockInheritedWidget>;

    fn inherited_type_id(&self) -> TypeId {
        TypeId::of::<Self::Data>()
    }

    fn inherited_data(&self) -> &Self::Data {
        &self.widget
    }

    fn child(&self) -> Widget {
        self.widget.mock.borrow().child()
    }

    fn needs_notify(&mut self) -> bool {
        self.widget.mock.borrow_mut().needs_notify()
    }
}
