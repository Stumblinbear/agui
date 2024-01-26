use std::rc::Rc;

use crate::{
    element::{
        mock::render::{MockRenderObject, MockRenderWidget},
        Element, ElementBuilder, ElementComparison,
    },
    render::object::RenderObject,
    unit::HitTest,
    widget::{IntoWidget, Widget},
};

pub mod build;
pub mod inherited;
pub mod render;

pub struct DummyWidget;

impl IntoWidget for DummyWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for DummyWidget {
    type Element = <MockRenderWidget as ElementBuilder>::Element;

    fn create_element(self: Rc<Self>) -> Element {
        let widget = MockRenderWidget::default();
        {
            let mut widget_mock = widget.mock.borrow_mut();

            widget_mock.expect_children().returning(Vec::default);

            widget_mock.expect_update().returning(|new_widget| {
                if new_widget.downcast::<DummyWidget>().is_some() {
                    ElementComparison::Changed
                } else {
                    ElementComparison::Invalid
                }
            });

            widget_mock
                .expect_create_render_object()
                .returning(|_| DummyRenderObject.into());

            widget_mock
                .expect_update_render_object()
                .returning(|_, _| {});
        }

        Rc::new(widget).create_element()
    }
}

pub struct DummyRenderObject;

impl From<DummyRenderObject> for RenderObject {
    fn from(_: DummyRenderObject) -> Self {
        let render_object = MockRenderObject::default();
        {
            let mut render_object_mock = render_object.mock.lock();

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
        render_object.into()
    }
}
