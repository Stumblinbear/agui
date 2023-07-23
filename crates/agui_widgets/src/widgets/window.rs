use agui_core::{
    unit::{Constraints, Size},
    widget::{BuildContext, LayoutContext, WidgetBuild, WidgetLayout, WidgetRef},
};
use agui_macros::LayoutWidget;

#[derive(LayoutWidget, Default)]
pub struct Window {
    pub title: String,
    pub size: Size,

    pub child: WidgetRef,
}

impl WidgetBuild for Window {
    type Child = WidgetRef;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        self.child.clone()
    }
}

impl WidgetLayout for Window {
    fn layout(&self, _: &mut LayoutContext<Self>, _: Constraints) -> Size {
        Size {
            width: self.size.width,
            height: self.size.height,
        }
    }
}
