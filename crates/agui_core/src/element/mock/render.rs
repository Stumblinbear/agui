use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::Arc,
};

use parking_lot::{Mutex, MutexGuard};

use crate::{
    element::{
        lifecycle::ElementLifecycle, render::ElementRender, widget::ElementWidget, Element,
        ElementBuilder, ElementComparison, ElementMountContext, ElementUnmountContext,
        RenderObjectCreateContext, RenderObjectUpdateContext,
    },
    render::object::{MockRenderObjectImpl, RenderObject},
    unit::HitTest,
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

#[derive(Clone, Default)]
pub struct MockRenderWidget {
    mock: Rc<RefCell<MockRenderElement>>,
}

impl MockRenderWidget {
    pub fn dummy() -> Widget {
        let widget = MockRenderWidget::default();
        {
            let mut mock = widget.mock();

            mock.expect_children().returning(Vec::default);

            mock.expect_update().returning(|new_widget| {
                if new_widget.downcast::<MockRenderWidget>().is_some() {
                    ElementComparison::Changed
                } else {
                    ElementComparison::Invalid
                }
            });

            mock.expect_create_render_object()
                .returning(|_| MockRenderObject::dummy());

            mock.expect_update_render_object().returning(|_, _| {});
        }
        widget.into_widget()
    }

    pub fn mock(&self) -> RefMut<MockRenderElement> {
        self.mock.borrow_mut()
    }
}

impl IntoWidget for MockRenderWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for MockRenderWidget {
    type Element = MockedElementRender;

    fn create_element(self: Rc<Self>) -> Element {
        Element::new_render(MockedElementRender::new(self))
    }
}

#[derive(Default)]
pub struct MockRenderObject {
    mock: Arc<Mutex<MockRenderObjectImpl>>,
}

impl MockRenderObject {
    pub fn dummy() -> RenderObject {
        let mock_render_object = MockRenderObject::default();
        {
            let mut render_object_mock = mock_render_object.mock();

            render_object_mock
                .expect_intrinsic_size()
                .returning(|_, _, _| 0.0);

            render_object_mock
                .expect_layout()
                .returning(|_, contraints| contraints.smallest());

            render_object_mock
                .expect_hit_test()
                .returning(|_, _| HitTest::Pass);

            render_object_mock.expect_paint().returning(|_| {});
        }
        mock_render_object.create()
    }

    pub fn mock(&self) -> MutexGuard<'_, MockRenderObjectImpl> {
        self.mock.lock()
    }

    pub fn create(self) -> RenderObject {
        RenderObject::new(
            Arc::into_inner(self.mock)
                .expect("cannot convert mock to render object as a reference is still held")
                .into_inner(),
        )
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
