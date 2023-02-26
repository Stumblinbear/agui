use agui_core::{
    render::{CanvasPainter, Paint},
    unit::Color,
    widget::{BuildContext, Children, PaintContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default)]
pub struct ColoredBox {
    pub color: Color,

    pub child: WidgetRef,
}

impl WidgetView for ColoredBox {
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
