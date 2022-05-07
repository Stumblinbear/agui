use agui_core::{
    canvas::paint::Paint,
    manager::widget::Widget,
    unit::{Rect, Shape},
    widget::{BuildContext, BuildResult, StatelessWidget},
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
