use agui_core::{
    render::{CanvasPainter, Paint},
    unit::Color,
    widget::{BuildContext, PaintContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default)]
pub struct ColoredBox {
    pub color: Color,

    pub child: WidgetRef,
}

impl WidgetView for ColoredBox {
    type Child = WidgetRef;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        self.child.clone()
    }

    fn paint(&self, _ctx: &mut PaintContext<Self>, mut canvas: CanvasPainter) {
        canvas.draw_rect(&Paint {
            color: self.color,

            ..Paint::default()
        });
    }
}
