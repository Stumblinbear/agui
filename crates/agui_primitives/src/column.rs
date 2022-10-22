use agui_core::{
    unit::{Layout, LayoutType, Units},
    widget::{BuildContext, BuildResult, LayoutContext, LayoutResult, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct Column {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<WidgetRef>,
}

impl WidgetView for Column {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::Column {
                spacing: self.spacing,
            },

            layout: Layout::clone(&self.layout),
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
        BuildResult::from(&self.children)
    }
}
