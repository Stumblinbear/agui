use agui_core::{
    unit::{Layout, LayoutType, Margin, Sizing},
    widget::{BuildContext, Children, LayoutContext, LayoutResult, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct Center {
    pub child: WidgetRef,
}

impl WidgetView for Center {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Fill,

                margin: Margin::center(),

                ..Layout::default()
            },
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }
}
