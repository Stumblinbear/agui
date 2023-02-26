use agui_core::{
    unit::{Constraints, Size},
    widget::{BuildContext, Children, LayoutContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Default)]
pub struct Window {
    pub title: String,
    pub size: Size,

    pub child: WidgetRef,
}

impl WidgetView for Window {
    fn layout(&self, _: &mut LayoutContext<Self>, _: Constraints) -> Size {
        Size {
            width: self.size.width,
            height: self.size.height,
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }
}
