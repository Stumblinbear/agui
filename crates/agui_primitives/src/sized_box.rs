use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Color, Layout, LayoutType, Size, Sizing, Units},
    widget::{BuildContext, Children, LayoutContext, LayoutResult, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct SizedBox {
    pub width: Option<f32>,
    pub height: Option<f32>,

    pub child: WidgetRef,
}

impl WidgetView for SizedBox {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Axis {
                    width: self.width.map_or(Units::Auto, Units::Pixels),
                    height: self.height.map_or(Units::Auto, Units::Pixels),
                },

                ..Layout::default()
            },
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }
}
