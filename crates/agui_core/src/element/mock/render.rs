use std::{cell::RefCell, rc::Rc};

use crate::{
    element::{
        render::ElementRender, widget::ElementWidget, ElementBuilder, ElementHitTestContext,
        ElementIntrinsicSizeContext, ElementLayoutContext, ElementMountContext, ElementType,
        ElementUnmountContext, ElementUpdate,
    },
    render::canvas::Canvas,
    unit::{Constraints, HitTest, IntrinsicDimension, Offset, Size},
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

    fn get_children(&self) -> Vec<Widget>;

    fn intrinsic_size<'ctx>(
        &self,
        ctx: ElementIntrinsicSizeContext<'ctx>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32;

    fn layout<'ctx>(&self, ctx: ElementLayoutContext<'ctx>, constraints: Constraints) -> Size;

    fn hit_test<'ctx>(
        &self,
        ctx: &'ctx mut ElementHitTestContext<'ctx>,
        position: Offset,
    ) -> HitTest;

    fn paint(&self, size: Size) -> Option<Canvas>;
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
    fn get_children(&self) -> Vec<Widget> {
        self.widget.mock.borrow().get_children()
    }

    fn intrinsic_size(
        &self,
        ctx: ElementIntrinsicSizeContext,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        self.widget
            .mock
            .borrow()
            .intrinsic_size(ctx, dimension, cross_extent)
    }

    fn layout(&self, ctx: ElementLayoutContext, constraints: Constraints) -> Size {
        self.widget.mock.borrow().layout(ctx, constraints)
    }

    fn hit_test<'ctx>(
        &self,
        ctx: &'ctx mut ElementHitTestContext<'ctx>,
        position: Offset,
    ) -> HitTest {
        self.widget.mock.borrow().hit_test(ctx, position)
    }

    fn paint(&self, size: Size) -> Option<Canvas> {
        self.widget.mock.borrow().paint(size)
    }
}
