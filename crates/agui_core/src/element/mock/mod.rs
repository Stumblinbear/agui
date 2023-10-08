use std::rc::Rc;

use crate::{
    unit::HitTest,
    widget::{IntoWidget, Widget},
};

use self::render::MockRenderWidget;

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
        let widget = MockRenderWidget::default();
        {
            let mut widget_mock = widget.mock.borrow_mut();

            widget_mock.expect_widget_name().returning(|| "DummyWidget");

            widget_mock.expect_get_children().returning(Vec::default);

            widget_mock.expect_intrinsic_size().returning(|_, _, _| 0.0);

            widget_mock
                .expect_layout()
                .returning(|_, _| Default::default());

            widget_mock
                .expect_hit_test()
                .returning(|_, _| HitTest::Pass);

            widget_mock.expect_paint().returning(|_| None);

            widget_mock.expect_update().returning(|new_widget| {
                if new_widget.downcast::<DummyWidget>().is_some() {
                    ElementUpdate::RebuildNecessary
                } else {
                    ElementUpdate::Invalid
                }
            });
        }

        Rc::new(widget).create_element()
    }
}
