use agui_core::{
    canvas::paint::Paint,
    unit::Shape,
    widget::{BuildContext, BuildResult, StatelessWidget, Widget},
};

#[derive(Debug, Default)]
pub struct Clip {
    pub anti_alias: bool,
    pub shape: Shape,
    pub child: Widget,
}

impl StatelessWidget for Clip {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        ctx.on_draw(|ctx, canvas| {
            let brush = canvas.new_brush(Paint {
                anti_alias: ctx.anti_alias,
                ..Paint::default()
            });

            canvas.start_layer(brush, ctx.shape.clone());
        });

        (&self.child).into()
    }
}
