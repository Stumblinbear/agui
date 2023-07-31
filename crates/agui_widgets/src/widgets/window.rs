use agui_core::{
    unit::{Constraints, Size},
    widget::{BuildContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Default)]
pub struct Window {
    pub title: String,
    pub size: Size,

    pub child: Option<Widget>,
}

impl WidgetLayout for Window {
    type Children = Widget;

    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Self::Children> {
        Vec::from_iter(self.child.clone())
    }

    fn layout(&self, _: &mut LayoutContext, _: Constraints) -> Size {
        Size {
            width: self.size.width,
            height: self.size.height,
        }
    }
}
