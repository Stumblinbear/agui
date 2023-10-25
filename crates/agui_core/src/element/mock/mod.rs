use std::rc::Rc;

use crate::{
    render::RenderObject,
    unit::HitTest,
    widget::{IntoWidget, Widget},
};

use self::render::{MockRenderObject, MockRenderWidget};

use super::{ElementBuilder, ElementType, ElementUpdate};

pub mod build;
pub mod proxy;
pub mod render;

pub struct DummyWidget;

impl IntoWidget for DummyWidget {
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl ElementBuilder for DummyWidget {
    fn create_element(self: Rc<Self>) -> ElementType {
        let widget = MockRenderWidget::new("DummyWidget");
        {
            let mut widget_mock = widget.mock.borrow_mut();

            widget_mock.expect_children().returning(Vec::default);

            widget_mock.expect_update().returning(|new_widget| {
                if new_widget.downcast::<DummyWidget>().is_some() {
                    ElementUpdate::RebuildNecessary
                } else {
                    ElementUpdate::Invalid
                }
            });

            widget_mock
                .expect_create_render_object()
                .returning(|| DummyRenderObject.into());

            widget_mock.expect_update_render_object().returning(|_| {});
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
                .expect_render_object_name()
                .returning(|| "DummyRenderObject");

            render_object_mock
                .expect_intrinsic_size()
                .returning(|_, _, _| 0.0);

            render_object_mock
                .expect_layout()
                .returning(|_, contraints| contraints.smallest());

            render_object_mock
                .expect_hit_test()
                .returning(|_, _| HitTest::Pass);

            render_object_mock.expect_paint().returning(|_| None);
        }
        render_object.into()
    }
}
