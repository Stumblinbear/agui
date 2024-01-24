use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    callback::CallbackId,
    element::{
        lifecycle::ElementLifecycle, widget::ElementWidget, ElementBuildContext, ElementBuilder,
        ElementCallbackContext, ElementComparison, ElementMountContext, ElementType,
        ElementUnmountContext,
    },
    widget::{IntoWidget, Widget},
};

#[mockall::automock]
#[allow(clippy::needless_lifetimes)]
pub trait ElementBuild {
    fn mount<'ctx>(&mut self, ctx: &mut ElementMountContext<'ctx>);

    fn unmount<'ctx>(&mut self, ctx: &mut ElementUnmountContext<'ctx>);

    fn update(&mut self, new_widget: &Widget) -> ElementComparison;

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
    fn into_widget<'life>(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockBuildWidget {
    type Element = MockedElementBuild;

    fn create_element(self: Rc<Self>) -> ElementType {
        ElementType::new_widget(MockedElementBuild::new(self))
    }
}

pub struct MockedElementBuild {
    widget: Rc<MockBuildWidget>,
}

impl MockedElementBuild {
    pub fn new(widget: Rc<MockBuildWidget>) -> Self {
        Self { widget }
    }
}

impl ElementLifecycle for MockedElementBuild {
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        self.widget.mock.borrow_mut().update(new_widget)
    }
}

impl ElementWidget for MockedElementBuild {
    type Widget = MockBuildWidget;

    fn widget(&self) -> &Rc<Self::Widget> {
        &self.widget
    }
}

impl crate::element::build::ElementBuild for MockedElementBuild {
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
