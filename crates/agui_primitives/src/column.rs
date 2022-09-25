use agui_core::{
    unit::{Layout, LayoutType, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};

#[derive(Debug, Default, PartialEq)]
pub struct Column {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<WidgetRef>,
}

impl WidgetBuilder for Column {
    fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
        BuildResult {
            layout_type: LayoutType::Column {
                spacing: self.spacing,
            },

            layout: Layout::clone(&self.layout),

            children: self.children.clone(),
        }
    }
}
