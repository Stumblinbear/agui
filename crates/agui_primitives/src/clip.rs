use agui_core::{
    render::{CanvasPainter, Paint},
    unit::{Rect, Shape},
    widget::{BuildContext, Children, PaintContext, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default)]
pub struct Clip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,

    pub child: WidgetRef,
}

impl WidgetView for Clip {
    fn build(&self, _ctx: &mut BuildContext<Self>) -> Children {
        (&self.child).into()
    }

    fn paint(&self, _ctx: &mut PaintContext<Self>, canvas: CanvasPainter) {
        let paint = Paint {
            anti_alias: self.anti_alias,
            ..Paint::default()
        };

        match self.rect {
            Some(rect) => canvas.start_layer_at(rect, &paint, self.shape.clone()),
            None => canvas.start_layer(&paint, self.shape.clone()),
        };
    }
}
