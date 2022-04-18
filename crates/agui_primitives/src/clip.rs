use agui_core::{
    canvas::paint::Paint,
    unit::{Rect, Shape},
    widget::{BuildContext, BuildResult, StatelessWidget, Widget},
};

#[derive(Debug, Default)]
pub struct Clip {
    pub rect: Option<Rect>,

    pub shape: Shape,
    pub anti_alias: bool,

    pub child: Widget,
}

impl StatelessWidget for Clip {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.on_draw(|ctx, canvas| {
            let brush = canvas.new_brush(Paint {
                anti_alias: ctx.anti_alias,
                ..Paint::default()
            });

            match ctx.rect {
                Some(rect) => canvas.start_layer_at(rect, brush, ctx.shape.clone()),
                None => canvas.start_layer(brush, ctx.shape.clone()),
            }
        });

        (&self.child).into()
    }
}
