use agui_core::{
    unit::{Layout, LayoutType, Margin, Sizing},
    widget::{BuildContext, BuildResult, LayoutContext, LayoutResult, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct Padding {
    pub padding: Margin,

    pub child: WidgetRef,
}

impl WidgetView for Padding {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Fill,

                margin: self.padding,

                ..Layout::default()
            },
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
        BuildResult::from(&self.child)
    }
}
