use std::{borrow::Cow, rc::Rc};

use agui_core::{
    unit::{Constraints, IntrinsicDimension, Size, TextStyle},
    widget::{IntoWidget, Widget},
};
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;

use crate::sized_box::SizedBox;

#[derive(InheritedWidget)]
pub struct TextLayoutController {
    pub delegate: Rc<dyn TextLayoutDelegate>,

    pub child: Option<Widget>,
}

impl InheritedWidget for TextLayoutController {
    fn get_child(&self) -> Widget {
        self.child
            .clone()
            .unwrap_or_else(|| SizedBox::shrink().into_widget())
    }

    fn should_notify(&self, other_widget: &Self) -> bool {
        !std::ptr::eq(
            Rc::as_ptr(&self.delegate) as *const _ as *const (),
            Rc::as_ptr(&other_widget.delegate) as *const _ as *const (),
        )
    }
}

pub trait TextLayoutDelegate {
    fn compute_intrinsic_size(
        &self,
        font_style: &TextStyle,
        text: Cow<'static, str>,
        dimension: IntrinsicDimension,
        cross_axis: f32,
    ) -> f32;

    fn compute_layout(
        &self,
        font_style: &TextStyle,
        text: Cow<'static, str>,
        constraints: Constraints,
    ) -> Size;
}
