use agui_core::{
    unit::{Layout, LayoutType, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};

#[derive(Debug, Default, PartialEq)]
pub struct Row {
    pub layout: Layout,

    pub spacing: Units,

    pub children: Vec<WidgetRef>,
}

impl WidgetBuilder for Row {
    fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
        BuildResult {
            layout_type: LayoutType::Row {
                spacing: self.spacing,
            },

            layout: Layout::clone(&self.layout),

            children: self.children.clone(),
        }
    }
}
