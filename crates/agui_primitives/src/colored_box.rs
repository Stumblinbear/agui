use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Color, Layout, LayoutType, Sizing},
    widget::{
        BuildContext, Children, LayoutContext, LayoutResult, PaintContext, WidgetRef, WidgetView,
    },
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
pub struct ColoredBox {
    pub color: Color,

    pub child: WidgetRef,
}

impl WidgetView for ColoredBox {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: Sizing::Fill,

                ..Layout::default()
            },
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(&self.child)
    }

    fn paint(&self, _ctx: &mut PaintContext<Self>, mut canvas: CanvasPainter) {
        canvas.draw_rect(&Paint {
            color: self.color,

            ..Paint::default()
        });
    }
}
