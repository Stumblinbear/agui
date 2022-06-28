use agui_core::{
    render::canvas::paint::Paint,
    unit::{Rect, Shape},
    widget::{BuildContext, BuildResult, Widget, WidgetBuilder},
};

#[derive(Debug, Default)]
pub struct Clip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,

    pub child: Widget,
}

impl WidgetBuilder for Clip {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.on_draw(|ctx, canvas| {
            let paint = Paint {
                anti_alias: ctx.anti_alias,
                ..Paint::default()
            };

            match ctx.rect {
                Some(rect) => canvas.start_layer_at(rect, &paint, ctx.shape.clone()),
                None => canvas.start_layer(&paint, ctx.shape.clone()),
            }
        });

        (&self.child).into()
    }
}
