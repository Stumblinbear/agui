use std::{borrow::Cow, rc::Rc};

use agui_core::{
    unit::{Constraints, FontStyle, IntrinsicDimension, Size},
    widget::{InheritedWidget, IntoChild, Widget},
};
use agui_macros::InheritedWidget;

#[derive(InheritedWidget, Default)]
pub struct TextLayoutController {
    pub delegate: Option<Rc<dyn TextLayoutDelegate>>,

    #[child]
    pub child: Option<Widget>,
}

impl InheritedWidget for TextLayoutController {
    fn should_notify(&self, other_widget: &Self) -> bool {
        match (&self.delegate, &other_widget.delegate) {
            (Some(delegate), Some(other_delegate)) => !std::ptr::eq(
                Rc::as_ptr(delegate) as *const _ as *const (),
                Rc::as_ptr(other_delegate) as *const _ as *const (),
            ),

            (Some(_), None) | (None, Some(_)) => true,

            _ => false,
        }
    }
}

impl TextLayoutController {
    pub fn new() -> Self {
        Self {
            delegate: None,

            child: None,
        }
    }

    pub fn with_delegate<D>(mut self, delegate: D) -> Self
    where
        D: TextLayoutDelegate + 'static,
    {
        self.delegate = Some(Rc::new(delegate));

        self
    }

    pub fn with_child(mut self, child: impl IntoChild) -> Self {
        self.child = child.into_child();

        self
    }
}

pub trait TextLayoutDelegate {
    fn compute_intrinsic_size(
        &self,
        font_style: &FontStyle,
        text: Cow<'static, str>,
        dimension: IntrinsicDimension,
        cross_axis: f32,
    ) -> f32;

    fn compute_layout(
        &self,
        font_style: &FontStyle,
        text: Cow<'static, str>,
        constraints: Constraints,
    ) -> Size;
}
