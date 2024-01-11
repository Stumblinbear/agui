use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    callback::CallbackId,
    element::{
        widget::ElementWidget, ElementBuildContext, ElementBuilder, ElementCallbackContext,
        ElementMountContext, ElementType, ElementUnmountContext, ElementUpdate,
    },
    widget::{IntoWidget, Widget},
};

#[allow(clippy::disallowed_types)]
#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait ElementBuild {
    fn mount<'ctx>(&mut self, ctx: &mut ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: &mut ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementUpdate;

    fn build<'ctx>(&mut self, ctx: &mut ElementBuildContext<'ctx>) -> Widget;

    fn call<'ctx>(
        &mut self,
        ctx: &mut ElementCallbackContext<'ctx>,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool;
}

#[derive(Default)]
pub struct MockBuildWidget {
    pub mock: Rc<RefCell<MockElementBuild>>,
}

impl IntoWidget for MockBuildWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockBuildWidget {
    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::new_widget(MockElement::new(self))
    }
}

struct MockElement {
    widget: Rc<MockBuildWidget>,
}

impl MockElement {
    pub fn new(widget: Rc<MockBuildWidget>) -> Self {
        Self { widget }
    }
}

impl ElementWidget for MockElement {
    fn update(&mut self, new_widget: &Widget) -> ElementUpdate {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl crate::element::build::ElementBuild for MockElement {
    fn build(&mut self, ctx: &mut ElementBuildContext) -> Widget {
        self.widget.mock.borrow_mut().build(ctx)
    }

    fn call(
        &mut self,
        ctx: &mut ElementCallbackContext,
        callback_id: CallbackId,
        arg: Box<dyn Any>,
    ) -> bool {
        self.widget.mock.borrow_mut().call(ctx, callback_id, arg)
    }
}
