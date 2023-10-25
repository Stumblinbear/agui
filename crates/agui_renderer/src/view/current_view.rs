use std::ops::Deref;

use agui_core::widget::Widget;
use agui_inheritance::InheritedWidget;
use agui_macros::InheritedWidget;

use crate::RenderViewId;

#[derive(InheritedWidget)]
pub struct CurrentRenderView {
    #[prop(skip, default)]
    pub id: RenderViewId,

    pub(crate) child: Widget,
}

impl Deref for CurrentRenderView {
    type Target = RenderViewId;

    fn deref(&self) -> &Self::Target {
        &self.id
    }
}

impl InheritedWidget for CurrentRenderView {
    fn child(&self) -> Widget {
        self.child.clone()
    }

    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        self.id != old_widget.id
    }
}
