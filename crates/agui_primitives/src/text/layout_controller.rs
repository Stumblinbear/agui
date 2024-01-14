use std::{borrow::Cow, sync::Arc};

use agui_core::{
    unit::{Constraints, IntrinsicDimension, Size, TextStyle},
    util::ptr_eq::PtrEqual,
    widget::Widget,
};
use agui_elements::inherited::InheritedWidget;
use agui_macros::InheritedWidget;

#[derive(InheritedWidget)]
pub struct TextLayoutController {
    pub delegate: Arc<dyn TextLayoutDelegate + Send + Sync + 'static>,

    pub child: Widget,
}

impl InheritedWidget for TextLayoutController {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    fn should_notify(&self, old_widget: &Self) -> bool {
        !self.delegate.is_exact_ptr(&old_widget.delegate)
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
