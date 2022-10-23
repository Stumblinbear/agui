use agui_core::{
    unit::{Layout, LayoutType, Size, Sizing, Units},
    widget::{BuildContext, BuildResult, LayoutContext, LayoutResult, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Default, PartialEq)]
pub struct Window {
    pub title: String,
    pub size: Size,

    pub child: WidgetRef,
}

impl WidgetView for Window {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Axis {
                    width: Units::Pixels(self.size.width),
                    height: Units::Pixels(self.size.height),
                },

                ..Layout::default()
            },
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
        BuildResult::from(&self.child)
    }
}
