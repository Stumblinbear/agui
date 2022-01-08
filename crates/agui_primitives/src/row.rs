use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{LayoutType, Units},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
    Ref,
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Row {
    pub layout: Ref<Layout>,

    pub spacing: Units,

    pub children: Vec<WidgetRef>,
}

impl WidgetBuilder for Row {
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        ctx.set_layout_type(
            LayoutType::Row {
                spacing: self.spacing,
            }
            .into(),
        );

        ctx.set_layout(Ref::clone(&self.layout));

        (&self.children).into()
    }
}